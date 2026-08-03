// SPDX-License-Identifier: MPL-2.0

//! The shared whiteout cache and whiteout-publish mechanics — `P1-25`/`P1-36`.
//!
//! This module owns the frozen meso-06 spec §4 `dir/whiteout.rs` surface on
//! [`OverlayFs`] and [`WhiteoutCache`]: the mount-scoped `WL` payload
//! ([`WhiteoutCache`], bounded to one cached workdir whiteout), the
//! capability-derived [`WhiteoutRepresentation`] (meso-01 `can_mknod_char` →
//! `CharDevice`, else `can_store_private_xattr` → `Xattr`; **no runtime
//! probe**, revision-01 override 9), the cached-item shape
//! ([`WhiteoutHandle`]), the private temp creation
//! ([`OverlayFs::create_whiteout_temp`]), the publish entry
//! ([`OverlayFs::publish_whiteout`], `P1-25`), and the short
//! `take`/`store`/`disable_sharing` slot protocol (`P1-36`). The sibling
//! `dir/remove.rs` (P1-26/27) and `dir/rename.rs` (P1-29/30) passes compose
//! `publish_whiteout` for the lower-backed removal and rename-source-cleanup
//! whiteouts; `create.rs`/`link.rs` consume the opaque/whiteout
//! replacement semantics without touching this module's payload.
//!
//! # The `WL` (Level 5) lock domain
//!
//! The single `WL` payload is `OverlayFs::whiteout_cache: Mutex<WhiteoutCache>`
//! (the Wave-3 seam in `mount/superblock.rs`). `WL` critical sections are the
//! **short slot operations only** — `take`/`store`/`disable_sharing` (pop /
//! push / flag) — and never contain BIO, sleeping allocation, underlying VFS
//! calls, callbacks, or waits (spec §8; Hazard 2). All fallible and
//! sleep-capable work — temp creation (`mknod`/`create` + the whiteout-marker
//! xattr write), the underlying `link`, and the workdir `rename` — runs
//! **outside** `WL` in the caller's sleep-capable `DIR` domain (spec §3 item
//! 6). The Mutex-vs-RwMutex question for this field is a deferred ledger item
//! (revision 01, override 9): the lock type is unchanged.
//!
//! # Representation derivation
//!
//! The whiteout physical form is derived, never probed (revision 01, override
//! 9; recorded dependency §11 item 2): `OverlayFs::whiteout_representation`
//! returns `CharDevice` when the meso-01 capability `can_mknod_char` is set,
//! else `Xattr` when `can_store_private_xattr` is set, else the defensive
//! `EOPNOTSUPP` (unreachable for a writable overlay with lowers per the
//! meso-01 revision-02 whiteout-capability mount gate). The representation is
//! deliberately **not** stored on the cache (no duplicate state; the enum
//! classifies the two closed physical forms of spec §5.1).
//!
//! # Invariants (spec §4/§8)
//!
//! - At most one cached whiteout (`cached: Option<WhiteoutHandle>`); a cached
//!   whiteout is a workdir object that is never a directory entry of any
//!   upper parent nor a `ReaddirIndex` source (BC-6 §57).
//! - `can_share_by_link == false` implies future publishes use rename-over
//!   move semantics (set once on `EMLINK`/`EOPNOTSUPP`; never re-enabled).
//! - A published whiteout is a visibility barrier, never an inode (BC-2
//!   §18.2): the publish entry only produces the upper object; the
//!   `HiddenByWhiteout(HiddenEvidence)` binding publication is the sibling
//!   recipe's inline seam composition.
//! - No `.unwrap()`/`.expect()` in any production path (hard invariant
//!   failures use the recorded `unreachable!`/error-return precedents).
//!
//! Visibility: the spec's `pub(super)` items are read through the packet
//! override ("the overlayfs ceiling `pub(in crate::fs::fs_impls::overlayfs)`
//! where the spec says `pub(super)` and cross-module reachability requires it
//! — apply the Wave-3 precedent"). [`WhiteoutCache`] and its constructor are
//! at the ceiling because the landed Wave-3 `OverlayFs::whiteout_cache` field
//! (`mount/superblock.rs`) and its initialization (`mount/build.rs`) name
//! them from sibling module trees; [`WhiteoutHandle`]/[`WhiteoutRepresentation`]
//! and the accessor/publish entries stay at the spec's `pub(super)` (visible
//! within the `dir` module tree only), matching spec §1 "Must Remain
//! Internal".

use crate::{
    fs::{
        file::{InodeMode, InodeType},
        fs_impls::overlayfs::mount::OverlayFs,
        vfs::{
            inode::{Inode, MknodType, RenameMode},
            xattr::{XattrName, XattrSetFlags},
        },
    },
    prelude::*,
};

/// The xattr name of the xattr-based whiteout marker (Linux `OVL_XATTR_XWHITEOUT`).
///
/// Written by the owning meso-06 operation (spec §5.1) on the zero-size
/// regular file of the `Xattr` representation. The suffix `whiteout` is a
/// known Overlay-private record (`metadata_security/xattr.rs`
/// `OVERLAY_PRIVATE_SUFFIXES`), so the name classifies as `Private` through
/// the meso-05 `OverlayXattrPolicy::classify`/`is_private` seam.
const WHITEOUT_XATTR_FULL_NAME: &str = "trusted.overlay.whiteout";

/// The marker value of the xattr-based whiteout (the single byte `"y"`).
///
/// The meso-02 whiteout reader is presence-based (accepted meso-02 spec; the
/// first byte `b'y'` is the Linux `OVL_XATTR_XWHITEOUT` value — recorded
/// dependency §11 item 2, confirmed against `projection/entry.rs`).
const WHITEOUT_MARKER_VALUE: &[u8] = b"y";

/// The target-name component of workdir whiteout temp names.
///
/// `create_whiteout_temp` takes no name argument (frozen signature), yet the
/// frozen naming seam `generate_workdir_temp_name(target_name, upper_parent)`
/// requires a target-name component; the cached whiteout is a generic
/// workdir resource — not a `(parent, name)` owner (BC-6 §57) — so a fixed
/// content-named component is used. Uniqueness comes from the seam's
/// composite (`#{name}#{parent_ino}#{serial}`): the workdir-root real ino
/// plus the per-mount saturating `workdir_temp_serial` make the name unique
/// per mount (P1-35 guarantees no cross-mount collision).
const WHITEOUT_TEMP_NAME_COMPONENT: &str = "whiteout";

/// The mount-scoped reusable whiteout cache — the `WL` (Level 5) payload.
///
/// Bounded to one reusable workdir whiteout (private staging; BC-6 §57) plus
/// the share-by-link flag. Invariants: at most one cached whiteout; a cached
/// whiteout is a workdir object that is never a directory entry of any upper
/// parent nor a `ReaddirIndex` source; `can_share_by_link == false` implies
/// future publishes use rename-over. The whiteout *representation* is NOT
/// stored here (revision 01, override 9 — no duplicate state): it is derived
/// on demand from the immutable meso-01 published capabilities via
/// [`OverlayFs::whiteout_representation`].
///
/// Owner/guard: `OverlayFs::whiteout_cache: Mutex<WhiteoutCache>` — the `WL`
/// domain, a sleep-capable `ostd::sync::Mutex` whose critical sections never
/// contain BIO/sleep/underlying calls/callbacks/waits (the cache-slot
/// protocol is spec §8). The Mutex-vs-RwMutex question is a deferred ledger
/// item (revision 01, override 9) — the lock type is unchanged.
///
/// Visibility: at the overlayfs ceiling (the spec's `pub(super)` read through
/// the packet override) because the landed Wave-3 `OverlayFs::whiteout_cache`
/// field (`mount/superblock.rs`) and its `WhiteoutCache::new()` construction
/// (`mount/build.rs`) name this type from sibling module trees (the
/// `copyup::coordination` precedent). The cache-slot fields stay private;
/// the only external surface is the constructor and the slot methods used by
/// this file's `publish_whiteout`.
#[derive(Debug)]
pub(in crate::fs::fs_impls::overlayfs) struct WhiteoutCache {
    /// The single reusable workdir whiteout (private staging); `None` when
    /// the slot is empty. Bounded to 1.
    cached: Option<WhiteoutHandle>,
    /// `true` initially; set `false` on `EMLINK`/`EOPNOTSUPP` (move
    /// semantics, Linux `no_shared_whiteout`, `dir.c:77-119`).
    can_share_by_link: bool,
}

impl WhiteoutCache {
    /// Constructs the empty cache slot (spec §4 initialization: `cached:
    /// None`, `can_share_by_link: true`).
    ///
    /// Called by the landed Wave-3 `OverlayFs::new` (`mount/build.rs`) under
    /// the cross-meso owner-extension rule (meso-06 spec §4.1) — no meso-01
    /// revision; the constructor is the single construction path outside this
    /// module.
    pub(in crate::fs::fs_impls::overlayfs) fn new() -> Self {
        Self {
            cached: None,
            can_share_by_link: true,
        }
    }

    /// Pops the cached whiteout handle, if any (the `WL` slot-pop).
    ///
    /// Short critical section only: no BIO/sleep/underlying call under `WL`
    /// (spec §8). The protocol takes before storing, so the slot is empty
    /// after a successful take.
    fn take(&mut self) -> Option<WhiteoutHandle> {
        self.cached.take()
    }

    /// Pushes a whiteout handle back into the cache (the `WL` slot-push).
    ///
    /// Bounded to one slot: the protocol pops before publishing and re-stores
    /// only the workdir original kept alive by the link path, so an occupied
    /// slot here is a protocol violation; the stale handle is dropped (its
    /// workdir object becomes recorded P3-09 residue, never a visible source)
    /// rather than exceeding the bound. Short critical section only (spec
    /// §8).
    fn store(&mut self, handle: WhiteoutHandle) {
        if self.cached.replace(handle).is_some() {
            warn!(
                "overlay whiteout cache slot occupied at store; the stale cached whiteout is \
                 dropped (P3-09 workdir-cleanup residue, never a visible source)"
            );
        }
    }

    /// Disables whiteout sharing by link (the `WL` fallback flag).
    ///
    /// Set on `EMLINK`/`EOPNOTSUPP` from the link path; once `false`, every
    /// future publish uses rename-over move semantics. Never re-enabled.
    /// Short critical section only (spec §8).
    fn disable_sharing(&mut self) {
        self.can_share_by_link = false;
    }
}

/// One cached or mutation-local workdir whiteout (the `WL` cached-item shape).
///
/// `inode` is the whiteout object — a char `0:0` device or a zero-size
/// regular file carrying the `trusted.overlay.whiteout` marker — and
/// `workdir_name` is its name in the workdir, needed for rename-over
/// publishes. Invariants: `workdir_name` is non-empty and unique (generated
/// via meso-04 `generate_workdir_temp_name`); the handle never outlives its
/// use in one mutation unless re-cached.
///
/// Owner/guard: owned by `WhiteoutCache::cached` or a mutation-local; the
/// strong inode pin keeps the workdir object alive. Visibility: the spec's
/// `pub(super)` freeze — visible within the `dir` module tree only; no
/// consumer outside `dir` names this shape (the sibling recipes consume
/// `publish_whiteout`).
#[derive(Debug)]
pub(super) struct WhiteoutHandle {
    /// The whiteout object (char `0:0` device or zero-size file + whiteout
    /// xattr); a strong pin keeps the workdir object alive.
    inode: Arc<dyn Inode>,
    /// Its name in the workdir; needed for rename-over publishes.
    workdir_name: String,
}

/// The closed set of physical whiteout forms (spec §5.1; `P1-25`/`P1-36`).
///
/// `CharDevice`: the classic whiteout — a char device `0:0` created by
/// workdir `mknod`. `Xattr`: a zero-size regular file carrying the
/// `trusted.overlay.whiteout` marker, requiring
/// `can_store_private_xattr`. Revision 01 (override 9): the choice is
/// DERIVED from the meso-01 published capabilities
/// (`OverlayFs::whiteout_representation()`: `can_mknod_char` → `CharDevice`,
/// else `can_store_private_xattr` → `Xattr`) — there is NO runtime probe and
/// NO per-mount cached copy. The enum (not a bare bool) classifies the closed
/// pair because the two forms carry different recipe behavior (mknod vs
/// create+xattr) and exactly matches the spec §5.1 pair (spec §4.5
/// justification).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum WhiteoutRepresentation {
    /// Classic whiteout: char device `0:0` (workdir mknod).
    CharDevice,
    /// Xattr whiteout: zero-size regular file + `trusted.overlay.whiteout`
    /// (needs `can_store_private_xattr`).
    Xattr,
}

impl OverlayFs {
    /// Returns the mount-scoped whiteout cache — the `WL` domain (Level 5)
    /// accessor (spec §4, frozen signature).
    ///
    /// The cache is the only `WL` payload; the slot protocol
    /// (`take`/`store`/`disable_sharing`) is the only `WL` critical section
    /// and never covers BIO/sleep/underlying calls (spec §8). The accessor
    /// exists so the sibling `dir` recipes can name the domain without
    /// touching the field; the cache slot itself is only ever manipulated by
    /// this file's `publish_whiteout`.
    pub(super) fn whiteout_cache(&self) -> &Mutex<WhiteoutCache> {
        &self.whiteout_cache
    }

    /// Derives the whiteout representation from the meso-01 published
    /// capabilities (revision 01, override 9 — no runtime probe).
    ///
    /// `can_mknod_char` → [`WhiteoutRepresentation::CharDevice`]; else
    /// `can_store_private_xattr` → [`WhiteoutRepresentation::Xattr`]; else
    /// the defensive `EOPNOTSUPP` (unreachable for a writable overlay with
    /// lowers per the meso-01 revision-02 whiteout-capability mount gate; §11
    /// item 2). A missing capability snapshot means the mount has no writable
    /// claim (the snapshot is probed at mount time for writable mounts only),
    /// so the defensive arm is `EROFS` — the same writable-state error the
    /// admission gate and the meso-04 `workdir_root` resolver use; both arms
    /// are unreachable for a published writable overlay (recorded realization,
    /// Creator report §5).
    fn whiteout_representation(&self) -> Result<WhiteoutRepresentation> {
        let capabilities = self.policy().upper_capabilities().ok_or_else(|| {
            Error::with_message(
                Errno::EROFS,
                "the overlay mount has no writable upper capability snapshot",
            )
        })?;
        if capabilities.can_mknod_char() {
            Ok(WhiteoutRepresentation::CharDevice)
        } else if capabilities.can_store_private_xattr() {
            Ok(WhiteoutRepresentation::Xattr)
        } else {
            Err(Error::with_message(
                Errno::EOPNOTSUPP,
                "the upper filesystem supports no whiteout form (neither char-device mknod \
                 nor private xattr)",
            ))
        }
    }

    /// Creates one private workdir whiteout temp outside `WL` (BIO-capable).
    ///
    /// `representation := whiteout_representation()?`; the unique temp name
    /// comes from the frozen meso-04 naming seam
    /// [`OverlayFs::generate_workdir_temp_name`] (with the fixed
    /// `WHITEOUT_TEMP_NAME_COMPONENT` — see the constant's doc; the cached
    /// whiteout is a generic workdir resource, not a `(parent, name)` owner).
    /// `CharDevice` → workdir `mknod(temp_name, mode 0, CharDevice(0))`;
    /// `Xattr` → workdir `create(temp_name, File, mode 0)` then
    /// `set_xattr("trusted.overlay.whiteout", "y", CREATE_OR_REPLACE)` — the
    /// marker write is this Meso's owning operation (spec §5.1; the name is
    /// verified through the meso-05 `OverlayXattrPolicy::is_private`
    /// classification as a `debug_assert!` hard invariant, and the `Xattr`
    /// path is gated by `can_store_private_xattr` through the representation
    /// derivation). On an xattr-write failure the created temp is removed
    /// best-effort (`cleanup_workdir_temp`) so no workdir residue outlives
    /// the failed creation (P3-09 obligation, never a visible source).
    fn create_whiteout_temp(&self) -> Result<WhiteoutHandle> {
        let representation = self.whiteout_representation()?;
        // The workdir root resolves through the single shared resolver
        // (`OverlayFs::workdir_root`, wave-4 round-2 repair item 5) — the
        // inline claims block is deleted.
        let workdir = self.workdir_root()?;
        let temp_name = self.generate_workdir_temp_name(WHITEOUT_TEMP_NAME_COMPONENT, &workdir);
        match representation {
            WhiteoutRepresentation::CharDevice => {
                let inode =
                    workdir.mknod(&temp_name, InodeMode::empty(), MknodType::CharDevice(0))?;
                Ok(WhiteoutHandle {
                    inode,
                    workdir_name: temp_name,
                })
            }
            WhiteoutRepresentation::Xattr => {
                // The zero-size regular file carries the whiteout marker
                // (Linux `OVL_XATTR_XWHITEOUT`); both the marker spelling and
                // the classification are the owning meso-06 operation (spec
                // §5.1). The representation derivation already gated this
                // branch on `can_store_private_xattr` (meso-01).
                debug_assert!(
                    self.xattr_policy().is_private(WHITEOUT_XATTR_FULL_NAME),
                    "the whiteout marker name must classify as an overlay-private record"
                );
                let inode = workdir.create(&temp_name, InodeType::File, InodeMode::empty())?;
                let marker_name = XattrName::try_from_full_name(WHITEOUT_XATTR_FULL_NAME)
                    .ok_or_else(|| {
                        Error::with_message(
                            Errno::EINVAL,
                            "invalid overlay whiteout marker xattr name",
                        )
                    })?;
                let mut marker_reader = VmReader::from(WHITEOUT_MARKER_VALUE).to_fallible();
                if let Err(err) = inode.set_xattr(
                    marker_name,
                    &mut marker_reader,
                    XattrSetFlags::CREATE_OR_REPLACE,
                ) {
                    // Best-effort temp cleanup on the pre-publication failure
                    // (spec §7.1 step-5 analog; the P3-09 obligation never
                    // becomes a visible entry).
                    let _ = self.cleanup_workdir_temp(&temp_name);
                    return Err(err);
                }
                Ok(WhiteoutHandle {
                    inode,
                    workdir_name: temp_name,
                })
            }
        }
    }

    /// Publishes a whiteout at `(upper_parent, name)` — `P1-25` (spec §4/§8).
    ///
    /// Obtains a whiteout (`WL` pop of the cached handle, or a fresh
    /// [`OverlayFs::create_whiteout_temp`] outside `WL`), then publishes at
    /// `(upper_parent, name)` per `replace_target`:
    ///
    /// - `None` (target absent) → `upper_parent.link(&whiteout.inode, name)`
    ///   keeps the workdir original, which is re-stored under `WL` (bounded
    ///   to 1). `EMLINK`/`EOPNOTSUPP` on the link path → `disable_sharing`
    ///   under `WL` and a retry with move semantics (rename-over; Linux
    ///   `no_shared_whiteout`, `dir.c:77-119`). When sharing is already
    ///   disabled the publish starts directly with the rename-over.
    /// - `Some(non-dir)` (target present) → `workdir.rename(temp_name,
    ///   upper_parent, name, Replace)`; the whiteout is consumed, no re-cache.
    /// - `Some(Dir)` (target present) →
    ///   `workdir.rename(temp_name, upper_parent, name, Exchange)` — the
    ///   displaced directory lands in the workdir at the temp name — then
    ///   best-effort workdir `rmdir` cleanup of the displaced dir
    ///   (clear-empty/rmdir paths); a cleanup failure is the recorded P3-09
    ///   obligation and never a visible namespace entry (the whiteout is
    ///   already published, so the semantic publish succeeded).
    ///
    /// The whiteout marker bytes are written by the owning operation inside
    /// `create_whiteout_temp` (the `Xattr` form), which `publish_whiteout`
    /// invokes for a fresh temp; a cached whiteout carries the marker from
    /// its creation, so every published object carries it before the link/
    /// rename (recorded realization, Creator report §5). Runs in the
    /// sleep-capable `DIR` domain of the caller; `WL` is held only for the
    /// short slot operations.
    pub(super) fn publish_whiteout(
        &self,
        upper_parent: &Arc<dyn Inode>,
        name: &str,
        replace_target: Option<InodeType>,
    ) -> Result<()> {
        // Step 1 — the `WL` cache-slot pop (spec §8): read `can_share_by_link`
        // and take the cached handle under `WL`, then release `WL` before any
        // fallible/BIO-capable work. The block scope drops the guard before
        // the temp creation below.
        let (cached, can_share_by_link) = {
            let mut cache = self.whiteout_cache().lock();
            let cached = cache.take();
            let can_share_by_link = cache.can_share_by_link;
            (cached, can_share_by_link)
        };

        // Step 2 — obtain the whiteout handle OUTSIDE `WL` (BIO-capable:
        // workdir mknod/create + the marker xattr write).
        let handle = match cached {
            Some(handle) => handle,
            None => self.create_whiteout_temp()?,
        };

        // Step 3 — publish at `(upper_parent, name)`. The workdir root is the
        // physical rename source; a missing writable claim is the EROFS gate
        // (the admission already passed for a live mutation, so this is the
        // defensive arm) — resolved through the single shared resolver
        // (`OverlayFs::workdir_root`, wave-4 round-2 repair item 5); the
        // inline claims block is deleted.
        let workdir = self.workdir_root()?;
        match replace_target {
            // Target absent: the link path keeps the workdir original for
            // reuse (share); a link that fails the share contract degrades to
            // move semantics.
            None => {
                if can_share_by_link {
                    match upper_parent.link(&handle.inode, name) {
                        Ok(()) => {
                            // Re-store the workdir original under `WL`
                            // (bounded to 1); the link succeeded, so the
                            // whiteout object is shared, not consumed.
                            self.whiteout_cache().lock().store(handle);
                            return Ok(());
                        }
                        Err(err) if matches!(err.error(), Errno::EMLINK | Errno::EOPNOTSUPP) => {
                            // Linux `no_shared_whiteout`: disable sharing and
                            // retry with rename-over (move semantics).
                            self.whiteout_cache().lock().disable_sharing();
                        }
                        Err(err) => return Err(err),
                    }
                }
                workdir.rename(
                    &handle.workdir_name,
                    upper_parent,
                    name,
                    RenameMode::Replace,
                )?;
                Ok(())
            }
            // Target present (non-dir): rename the whiteout over it
            // (`Replace`); the whiteout is consumed, never re-cached.
            Some(target_type) if !target_type.is_directory() => {
                workdir.rename(
                    &handle.workdir_name,
                    upper_parent,
                    name,
                    RenameMode::Replace,
                )?;
                Ok(())
            }
            // Target present (dir): `Exchange` swaps the whiteout into the
            // name and the displaced directory into the workdir; the displaced
            // dir is then cleaned up best-effort (clear-empty/rmdir paths).
            // The whiteout is consumed, never re-cached.
            Some(_) => {
                workdir.rename(
                    &handle.workdir_name,
                    upper_parent,
                    name,
                    RenameMode::Exchange,
                )?;
                if let Err(cleanup_err) = workdir.rmdir(&handle.workdir_name) {
                    warn!(
                        "overlay whiteout publish: workdir cleanup of the displaced directory \
                         {:?} failed (P3-09 residue, never a visible source): {:?}",
                        handle.workdir_name, cleanup_err
                    );
                }
                Ok(())
            }
        }
    }
}
