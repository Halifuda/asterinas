// SPDX-License-Identifier: MPL-2.0

//! The merged-directory readdir index meso (`P0-13`/`P0-14`/`P0-15`/`P1-31`).
//!
//! Single flat module file (user-ruled revision 01, override 1) implementing
//! the frozen meso-03 spec surface: the `FileOps::readdir_at` VFS entry for
//! [`OverlayInode`] (superseding the meso-02 gate stub — the current tree
//! carries no stub, so this file is the real entry), the
//! `invalidate_readdir_index` P1-31 seam, the meso-06 decision seams
//! (`readdir_index_insert` / `readdir_index_remove`) and the
//! `visible_child_count` seam, the per-directory [`ReaddirIndex`] payload
//! (`entries` / `validity` / `next_cookie` / `tombstone_count`), and the
//! source-observation methods (`readdir_sequence` / `collect_layer_names`).
//!
//! The index is the first source for visible names (BC-3 §26.1): exactly one
//! current `ReaddirIndex` per overlay directory (`OverlayInode::readdir_index`,
//! `Some` iff directory); cookies are monotonic and never reused (`1`/`2`
//! reserved for `.`/`..`); the `validity` state machine is two-state only —
//! no `version` field exists (user ruling, override 3).
//!
//! Lock contract (spec §3): `DIR -> INODE(facts, brief) -> INODE(readdir_index)`;
//! no `INODE` guard is ever held across an underlying call; the visitor is
//! invoked under `DIR` and the sleep-capable index lock (Hazard 3). This Meso
//! acquires nothing above `INODE` (no `CUL`, `WL`, `UPPER`, or `MOUNT`).
//!
//! `..` identity note (P0-15, escalated in the Creator report): the frozen
//! primary route of `overlay_parent_object_id` (spec §4) projects the real
//! parent through `IdentityPolicy::project_object_id`, which is not reachable
//! at the overlayfs ceiling (frozen at `pub(super)` inside `projection`, and
//! `OverlayObjectId`'s fields are projection-private), so the serve loop
//! applies the spec's documented §5.3 item-3 fallback:
//! `d_ino("..") == d_ino(".")`.

use hashbrown::HashSet;

use super::{
    mount::OverlayFs,
    projection::{OverlayInode, OverlayObjectFacts, PositiveKind, RealObject},
};
use crate::{
    fs::{
        file::InodeType,
        utils::DirentVisitor,
        vfs::{
            inode::{FileOps, Inode},
            path::is_dot_or_dotdot,
        },
    },
    prelude::*,
};

/// The Overlay continuation cookie of one visible directory position.
///
/// Monotonic in visible order, strictly increasing, and never reused for a
/// different logical position (I4): `.`/`..` own the reserved cookies `1`/`2`,
/// real entries start at `3`, and `next_cookie` is a never-decreasing
/// high-water mark that survives rebuilds. The cookie is emitted as the
/// user-space `d_off` (procfs-slot convention, spec §5.2).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct ReaddirCookie(u64);

/// The serve-or-rebuild state of the index (spec §4; two-state only — no
/// `version`, no third "building" state; user ruling, override 3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ReaddirIndexValidity {
    /// The `Visible` entries are the complete current visible sequence (I1).
    Valid,
    /// Serving is refused; the next readdir rebuilds (I1; the conservative
    /// P1-31 outcome and the never-publish-partial fallback).
    NeedsRebuild,
}

/// The `INODE`-protected per-directory merged-readdir index payload
/// (spec §4; BC-3 §26.1).
///
/// `entries` are ordered by ascending `cookie` (cookie order == visible
/// order, I2); tombstones keep their old cookie slot, so old-cookie
/// continuation survives deletions. `validity == Valid` implies the `Visible`
/// entries are complete (I1); `next_cookie` is the no-reuse high-water mark
/// (I4); `tombstone_count` drives eager compaction at `>= live_count`
/// (`entries.len() == tombstone_count + live_count`; array ≤ 2× live —
/// override 4, satisfies BC-3 §33 no-unbounded-tombstones by construction).
///
/// No `Debug` derive (spec §4 derive markers are shape hints): the `Visible`
/// variant pins an `Arc<OverlayInode>`, which deliberately has no `Debug`
/// impl (projection/inode.rs dev note) — the same unsatisfiable-derive case
/// as `OverlayInode` itself.
///
/// Owner/guard: `OverlayInode::readdir_index: Option<Mutex<ReaddirIndex>>`
/// (level-4 `INODE` domain, sleep-capable); the lock protects exactly this
/// payload.
pub(super) struct ReaddirIndex {
    /// Visible + tombstone slots in ascending cookie order (I2).
    entries: Vec<ReaddirIndexEntry>,
    /// Serve-or-rebuild state (I1).
    validity: ReaddirIndexValidity,
    /// Cookie high-water mark; survives rebuilds; never decreases (I4).
    next_cookie: ReaddirCookie,
    /// Number of `Tombstone` slots; eager compaction when `>= live_count`.
    tombstone_count: usize,
}

/// One slot of the index (spec §4; override 4 — enum, not a flagged struct).
///
/// The variant types make "a tombstone never holds a strong pin" a
/// compile-time fact (`Tombstone` cannot contain `Arc<OverlayInode>`); a
/// flagged struct would admit a live entry with no inode that only runtime
/// checks could reject. `name`/`cookie` appear in both variants; shared
/// projection is via `match` (one concept, one type — no named intermediate).
///
/// No `Debug` derive: `Arc<OverlayInode>` is not `Debug` (shape hint).
pub(super) enum ReaddirIndexEntry {
    /// A live entry: strong pin (I7); `d_off == cookie` (procfs-slot
    /// convention, spec §5.2); `d_ino` source (I3); `type_` frozen at scan
    /// time and emitted as `d_type`.
    Visible {
        name: String,
        cookie: ReaddirCookie,
        inode: Arc<OverlayInode>,
        type_: InodeType,
    },
    /// A removed entry: reserved cookie + `Weak` inode only — never pins the
    /// removed object (override 4); enables same-object revive in place
    /// (`insert_visible`) and old-cookie continuation after deletion.
    Tombstone {
        name: String,
        cookie: ReaddirCookie,
        inode: Weak<OverlayInode>,
    },
}

impl FileOps for OverlayInode {
    /// The VFS readdir entry (spec §4; supersedes the meso-02 gate stub).
    ///
    /// Flow (top-down reading): guard directory type → acquire the payload-less
    /// `DIR` transaction lock → brief `facts_snapshot()` (`DIR -> INODE`) →
    /// `ensure_readdir_index` (serve a `Valid` index, or rebuild out-of-lock
    /// and publish a complete sequence) → serve: `first_entry_after(input)`
    /// via `partition_point`, then walk the entries skipping `Tombstone`
    /// slots and visiting `Visible` entries only, with `.`/`..` synthesis,
    /// `d_ino` application, and visitor-stop handling (Case 1-3, spec §5.2
    /// procfs-slot convention).
    fn readdir_at(&self, offset: usize, visitor: &mut dyn DirentVisitor) -> Result<usize> {
        // Case 5: readdir is supported on overlay directories only (the VFS
        // `InodeHandle`/`getdents` gates this too; the impl keeps the guard).
        let dir = self.dir().ok_or_else(|| Error::new(Errno::ENOTDIR))?;
        // The whole readdir transaction runs under the payload-less `DIR`
        // transaction lock: observation, build, and publication are
        // serialized per directory (BC-3 §32; one-`DIR` rule, BC-2 §23/§24).
        let _dir_guard = dir.lock();
        // Brief `INODE` facts snapshot (`DIR -> INODE`; the guard is released
        // before any lock-free use).
        let facts = self.facts_snapshot();
        // Serve-or-rebuild under the same `DIR` transaction (P0-13/P0-14).
        self.ensure_readdir_index(&facts)?;
        // Serve under the sleep-capable index `INODE` lock (Hazard 3).
        let index = self
            .readdir_index()
            .ok_or_else(|| Error::new(Errno::ENOTDIR))?;
        let index = index.lock();
        // `usize -> u64` is always lossless (Hazard 5; spec §5.2).
        let input_cookie = ReaddirCookie(offset as u64);
        let mut last_visited: Option<ReaddirCookie> = None;
        // The returned delta is `last_visited_cookie - input`; a visited
        // cookie is always `> input`, so the subtraction cannot underflow
        // (Hazard 5). `u64 -> usize` uses `try_from` (all supported arches
        // are 64-bit — recorded platform assumption).
        let delta_fn = |last_visited: Option<ReaddirCookie>| -> usize {
            let delta = match last_visited {
                Some(last) => last.0 - input_cookie.0,
                None => 0,
            };
            usize::try_from(delta).unwrap_or(usize::MAX)
        };
        // Reserved head cookies (I4): `.` (1) and `..` (2) are synthesized
        // special entries (BC-3 §28.1) and never appear in the `entries`
        // array.
        if input_cookie < ReaddirCookie(1) {
            // `.` carries this directory's projected identity (I3).
            if visitor.visit(".", self.ino(), InodeType::Dir, 1).is_err() {
                // Case 3: the first candidate failed; nothing was consumed.
                return Ok(delta_fn(None));
            }
            last_visited = Some(ReaddirCookie(1));
        }
        if input_cookie < ReaddirCookie(2) {
            // `..` carries the Overlay-parent identity (I3). This wave the
            // frozen projection route (spec §4 `overlay_parent_object_id`) is
            // blocked at the overlayfs ceiling — see the module doc and the
            // Creator-report escalation — so the documented §5.3 item-3
            // fallback applies: `d_ino("..") == d_ino(".")` (the directory's
            // own precomputed `object_id`, read through `Inode::ino()`).
            if visitor.visit("..", self.ino(), InodeType::Dir, 2).is_err() {
                return Ok(delta_fn(last_visited));
            }
            last_visited = Some(ReaddirCookie(2));
        }
        // Real entries (cookies >= 3): the first cookie above the input, then
        // walk in cookie order skipping `Tombstone` slots (I1) and visiting
        // `Visible` entries only (I2).
        let start = index
            .first_entry_after(input_cookie)
            .unwrap_or(index.entries.len());
        for entry in &index.entries[start..] {
            let ReaddirIndexEntry::Visible {
                name,
                cookie,
                inode,
                type_,
            } = entry
            else {
                // A tombstone is a traversal-skipped placeholder, never a
                // visible entry (I1).
                continue;
            };
            let d_off = match usize::try_from(cookie.0) {
                Ok(d_off) => d_off,
                // Unreachable on supported 64-bit targets (Hazard 5 recorded
                // platform assumption); a cookie beyond `usize` cannot be
                // served.
                Err(_) => break,
            };
            // `d_ino` derives from the shared identity policy (I3): the child
            // inode's precomputed `object_id` (`Inode::ino()`).
            if visitor.visit(name, inode.ino(), *type_, d_off).is_err() {
                // Case 3: the visitor stopped (e.g. user buffer full); the
                // error is not propagated (BC-3 §33; ext2 precedent, spec
                // §5.1); the consumed delta is returned.
                break;
            }
            last_visited = Some(*cookie);
        }
        // Case 1/2/3: the delta lands the per-FD offset exactly on the next
        // unvisited cookie (`Ok(0)` = end of sequence).
        Ok(delta_fn(last_visited))
    }
}

impl OverlayInode {
    /// P1-31 invalidation seam: marks the index `NeedsRebuild` (Case 7).
    ///
    /// Called by the namespace-mutation meso (06) — and by meso-04 after a
    /// directory authority transition that changes the visible source set —
    /// under this directory's `DIR`, after the underlying namespace commit
    /// and before `DIR` release. The seam takes only the index `INODE` lock
    /// (spec §3 inlet); no `version` exists to bump (user ruling, override 3).
    pub(super) fn invalidate_readdir_index(&self) {
        if let Some(index) = self.readdir_index() {
            index.lock().validity = ReaddirIndexValidity::NeedsRebuild;
        }
    }

    /// Meso-06 decision seam: fine-grained insert of a freshly visible name
    /// (create/mkdir/mknod/symlink/link publication, meso-06 spec §5.2).
    ///
    /// Frozen decision rule (meso-06 §5.2, meso-03-owned): the fine-grained
    /// insert runs only when the parent index is `Valid` AND the parent is
    /// upper-only (`facts.kind == Single`, `upper.is_some()`,
    /// `lowers.is_empty()`); every other case marks `NeedsRebuild`
    /// (conservative floor, BC-3 §27.1 — a merged/lower-backed parent or a
    /// stale index cannot provably keep the cookie order). Wave-4 repair item
    /// 13: the `insert_visible` verdict is consumed — a same-object revive
    /// keeps the index `Valid`; a fresh CREATE append (whose end-of-order
    /// position this seam cannot prove) falls back to `NeedsRebuild`. The
    /// caller (meso 06) holds this directory's `DIR`; the seam snapshots the
    /// facts briefly (INODE, released) and then takes the index `INODE` lock
    /// (intra-INODE order: facts before index, never simultaneous — spec §3).
    pub(super) fn readdir_index_insert(
        &self,
        name: &str,
        inode: Arc<OverlayInode>,
        type_: InodeType,
    ) {
        let facts = self.facts_snapshot();
        let Some(index) = self.readdir_index() else {
            return;
        };
        let mut index = index.lock();
        if index.validity == ReaddirIndexValidity::Valid
            && facts.kind() == PositiveKind::Single
            && facts.upper().is_some()
            && facts.lowers().is_empty()
        {
            // Wave-4 repair item 13: the revive-vs-create result is consumed
            // at the seam boundary. A same-object revive keeps the index
            // `Valid` (same cookie slot, provably same position); a fresh
            // CREATE append cannot be proven end-of-order here (the seam has
            // no position evidence), so it falls back to the conservative
            // `NeedsRebuild` floor (BC-3 §27.1) instead of trusting the
            // append position.
            if !index.insert_visible(name, inode, type_) {
                index.validity = ReaddirIndexValidity::NeedsRebuild;
            }
        } else {
            index.validity = ReaddirIndexValidity::NeedsRebuild;
        }
    }

    /// Meso-06 decision seam: fine-grained removal of a hidden/removed name
    /// (unlink/rmdir publication, meso-06 spec §5.2).
    ///
    /// Frozen decision rule (meso-06 §5.2): when the index is `Valid` the
    /// tombstone rule applies — the entry is converted to a `Tombstone` in
    /// place, keeping its cookie slot reserved (I4); a `NeedsRebuild` index
    /// stays as-is (the next readdir rebuilds wholesale). Wave-4 repair item
    /// 13: the `remove_visible` verdict is consumed — a `Valid` index that
    /// cannot tombstone the removed name violates its completeness claim, so
    /// the conservative `NeedsRebuild` floor applies.
    pub(super) fn readdir_index_remove(&self, name: &str) {
        let Some(index) = self.readdir_index() else {
            return;
        };
        let mut index = index.lock();
        if index.validity == ReaddirIndexValidity::Valid {
            // Wave-4 repair item 13: the removal result is consumed — a
            // `Valid` index that cannot tombstone the removed name violates
            // its completeness claim, so the conservative rebuild floor
            // applies (BC-3 §27.1).
            if !index.remove_visible(name) {
                index.validity = ReaddirIndexValidity::NeedsRebuild;
            }
        }
    }

    /// Meso-06 visible-emptiness seam (P1-27 rmdir; meso-06 spec §4.1).
    ///
    /// Ensures the index is `Valid` (rebuild under the same `DIR` transaction
    /// — the caller holds it) and counts the `Visible` entries (the real
    /// children; `.`/`..` are synthesized heads and never appear in the
    /// `entries` array).
    pub(super) fn visible_child_count(&self, facts: &OverlayObjectFacts) -> Result<usize> {
        self.ensure_readdir_index(facts)?;
        let index = self.readdir_index().ok_or_else(|| {
            Error::with_message(Errno::ENOTDIR, "the overlay inode is not a directory")
        })?;
        let index = index.lock();
        Ok(index
            .entries
            .iter()
            .filter(|entry| matches!(entry, ReaddirIndexEntry::Visible { .. }))
            .count())
    }

    /// Returns the per-directory index lock carrier (`Some` iff directory).
    ///
    /// The private index-lock accessor (spec §4): the `INODE`-domain
    /// `OverlayInode::readdir_index` payload, level 4, sleep-capable.
    fn readdir_index(&self) -> Option<&Mutex<ReaddirIndex>> {
        self.readdir_index.as_ref()
    }

    /// Ensures the directory's index is `Valid` under the caller's `DIR`
    /// transaction (spec §4; the build leg of P0-13/P0-14).
    ///
    /// A brief index `INODE` lock serves a `Valid` index; otherwise the lock
    /// is released and the source observation (`readdir_sequence`) runs
    /// out-of-lock (it may sleep on underlying reads — I8). After the
    /// released-lock segment the facts snapshot is re-checked — the sole
    /// race-revalidation signal, since no index `version` exists (user
    /// ruling, override 3) — and on mismatch the scan is discarded,
    /// `NeedsRebuild` is kept, and the rebuild retries once (Hazard 4); a
    /// persistent mismatch never publishes (I6). The complete sequence is
    /// published only under the index lock (I6); a failed scan leaves the
    /// index `NeedsRebuild` and never tears down a previously `Valid` index
    /// (the rebuild path replaces only on success — Case 4).
    fn ensure_readdir_index(&self, facts: &OverlayObjectFacts) -> Result<()> {
        let index = self.readdir_index().ok_or_else(|| {
            Error::with_message(Errno::ENOTDIR, "the overlay inode is not a directory")
        })?;
        {
            let index = index.lock();
            if index.validity == ReaddirIndexValidity::Valid {
                return Ok(());
            }
        }
        // Rebuild out-of-lock: no `INODE` guard is held across the
        // underlying reads (I8).
        let mut scan_facts = facts.clone();
        let mut retried = false;
        let sequence = loop {
            let sequence = self.readdir_sequence(&scan_facts)?;
            // Revalidate the facts snapshot after the released-lock segment
            // (Hazard 4 / BC-8 §3): the snapshot is compared on its
            // observable layer composition (kind + per-layer fsid/real-ino),
            // the only equality signals available at the overlayfs ceiling.
            let revalidated = self.facts_snapshot();
            let unchanged = {
                let same_kind = scan_facts.kind() == revalidated.kind();
                let same_upper = match (scan_facts.upper(), revalidated.upper()) {
                    (Some(left), Some(right)) => {
                        left.fsid() == right.fsid()
                            && left.real_inode().ino() == right.real_inode().ino()
                    }
                    (None, None) => true,
                    _ => false,
                };
                let same_lowers =
                    scan_facts.lowers().len() == revalidated.lowers().len()
                        && scan_facts.lowers().iter().zip(revalidated.lowers()).all(
                            |(left, right)| {
                                left.fsid() == right.fsid()
                                    && left.real_inode().ino() == right.real_inode().ino()
                            },
                        );
                same_kind && same_upper && same_lowers
            };
            if unchanged {
                break sequence;
            }
            if retried {
                // Persistent mismatch: a copy-up transition kept racing the
                // rebuild; never publish a stale index (I6) — leave
                // `NeedsRebuild` and surface the refusal (recorded
                // incidental deviation: the frozen "scan's last error" has
                // no value when both scans succeeded; `EIO` is the tree's
                // consistency-refusal convention — see the Creator report).
                return Err(Error::with_message(
                    Errno::EIO,
                    "the overlay directory facts changed while the readdir index was being rebuilt",
                ));
            }
            retried = true;
            scan_facts = revalidated;
        };
        // Publish the complete sequence under the index `INODE` lock (I6):
        // `rebuild` reads old cookies from the surviving entries and keeps
        // or allocates them per the frozen keep-rule (I4).
        index.lock().rebuild(sequence);
        Ok(())
    }

    /// Observes the current visible sequence of this directory from the
    /// pinned layer real objects (spec §4; the P0-13/P0-14 build leg).
    ///
    /// Merged (`facts.kind() == Merged`): enumerate the upper (when present),
    /// then each lower top→bottom, unless the upper is opaque
    /// (`is_opaque_directory()` — the opaque barrier stops the downward
    /// merge, BC-3 §28.2). Single (`facts.kind() == Single`): enumerate the
    /// single visible source (the upper, or `lowers[0]`). Per new name:
    /// `OverlayFs::lookup_binding` resolves the merged visibility — a
    /// positive binding contributes the shared [`OverlayInode`] (and its
    /// scan-time `type_`); a negative binding (whiteout/opaque evidence) is
    /// recorded in the local `seen` set and skipped. Dedup key is the
    /// visible name (I2). `.`/`..` are never scanned (I5). The
    /// `seen: HashSet<String>` and the returned
    /// `Vec<(String, Arc<OverlayInode>, InodeType)>` are pure locals / an
    /// anonymous tuple reuse of final payload types (spec §4.5). Runs
    /// out-of-lock inside the caller's `DIR` transaction; the caller
    /// re-validates the facts snapshot before publication (Hazard 4).
    fn readdir_sequence(
        &self,
        facts: &OverlayObjectFacts,
    ) -> Result<Vec<(String, Arc<OverlayInode>, InodeType)>> {
        // Bind the mount Arc before downcasting: the `Inode::fs()` upgrade
        // returns an owned `Arc<dyn FileSystem>` whose temporary would drop
        // before the borrowed `&OverlayFs` could be used (a live OverlayInode
        // keeps its mount alive — meso-02 §3.5 item 4).
        let fs = self.fs();
        let fs = fs.downcast_ref::<OverlayFs>().ok_or_else(|| {
            Error::with_message(
                Errno::EIO,
                "the overlay inode is not backed by an overlay mount",
            )
        })?;
        // The layers to enumerate in scan order (spec §4). The layer list is
        // a local (not a named type): `Single` -> the single visible source;
        // `Merged` -> the upper (when present) then the lowers top→bottom,
        // unless the upper is opaque.
        let layers: Vec<&RealObject> = match facts.kind() {
            PositiveKind::Single => {
                let source = match facts.upper() {
                    Some(upper) => upper,
                    // The `upper.is_some() || !lowers.is_empty()` facts
                    // invariant (meso-02 spec §4) guarantees `lowers[0]`.
                    None => &facts.lowers()[0],
                };
                vec![source]
            }
            PositiveKind::Merged => {
                let mut layers = Vec::new();
                match facts.upper() {
                    Some(upper) => {
                        layers.push(upper);
                        if !upper.is_opaque_directory()? {
                            layers.extend(facts.lowers());
                        }
                    }
                    None => layers.extend(facts.lowers()),
                }
                layers
            }
        };
        let mut seen = HashSet::new();
        let mut sequence = Vec::new();
        for layer in layers {
            for name in self.collect_layer_names(layer)? {
                // Dedup by visible name (I2): a name observed in an upper
                // layer is never re-emitted from a lower layer, and a
                // whiteout/opaque-hidden name is recorded and skipped.
                if !seen.insert(name.clone()) {
                    continue;
                }
                // Per-name visibility resolution: a positive binding
                // contributes the shared Overlay inode; a negative binding
                // (whiteout/opaque evidence) hides the name (P0-08..P0-11).
                if let Some(inode) = fs.lookup_binding(facts, &name)?.into_inode() {
                    sequence.push((name, inode, inode.type_()));
                }
            }
        }
        Ok(sequence)
    }

    /// Delegates the underlying readdir of one pinned real layer directory
    /// (spec §4; P0-13 "delegate").
    ///
    /// Collects the non-dot names in underlying enumeration order; underlying
    /// errors propagate (Case 4). `.`/`..` are never scanned (I5). No Overlay
    /// lock is held across the underlying call (I8); the caller's `DIR`
    /// transaction covers it.
    fn collect_layer_names(&self, layer: &RealObject) -> Result<Vec<String>> {
        let mut names = Vec::new();
        layer.real_inode().readdir_at(0, &mut names)?;
        names.retain(|name| !is_dot_or_dotdot(name));
        Ok(names)
    }
}

impl ReaddirIndex {
    /// Constructs the empty initial index (spec §4).
    ///
    /// `entries` empty, `validity == NeedsRebuild`, `next_cookie ==
    /// ReaddirCookie(3)` (cookies `1`/`2` are reserved for `.`/`..`),
    /// `tombstone_count == 0`. The Wave-3 `OverlayInode::readdir_index` seam
    /// initializes every directory carrier with this constructor.
    pub(super) fn new() -> Self {
        Self {
            entries: Vec::new(),
            validity: ReaddirIndexValidity::NeedsRebuild,
            next_cookie: ReaddirCookie(3),
            tombstone_count: 0,
        }
    }

    /// Rebuilds the index from a complete visible sequence (spec §4; I4/I6).
    ///
    /// Cookie assignment (I4): keep a name's previous cookie iff the name was
    /// `Visible` before AND the old entry's inode is the same logical object
    /// as the new binding (`Arc::ptr_eq`) AND the previous cookie is above
    /// the last assigned cookie (greedy order-preserving); every other
    /// appearance allocates a fresh cookie from the never-decreasing
    /// `next_cookie`. Drops ALL tombstones and resets `tombstone_count`; sets
    /// `validity = Valid`. Never renumbers already-exposed cookies. Old
    /// cookies are read from the surviving entries while the scan runs — no
    /// separate capture map (override 4).
    fn rebuild(&mut self, sequence: Vec<(String, Arc<OverlayInode>, InodeType)>) {
        let mut entries = Vec::with_capacity(sequence.len());
        // `.`/`..` own cookies 1/2; a kept cookie must stay above the last
        // assigned cookie so the array remains in ascending cookie order
        // (I2).
        let mut last_assigned = ReaddirCookie(2);
        for (name, inode, type_) in sequence {
            let previous = self.entries.iter().find_map(|old| match old {
                ReaddirIndexEntry::Visible {
                    name: old_name,
                    cookie: old_cookie,
                    inode: old_inode,
                    ..
                } if old_name == &name && Arc::ptr_eq(old_inode, &inode) => Some(*old_cookie),
                _ => None,
            });
            let cookie = match previous {
                Some(previous) if previous > last_assigned => previous,
                _ => {
                    let fresh = self.next_cookie;
                    // Checked/saturating arithmetic: the cookie space cannot
                    // be exhausted by any real directory; the high-water mark
                    // saturates instead of wrapping (I4).
                    self.next_cookie = ReaddirCookie(self.next_cookie.0.saturating_add(1));
                    fresh
                }
            };
            last_assigned = cookie;
            entries.push(ReaddirIndexEntry::Visible {
                name,
                cookie,
                inode,
                type_,
            });
        }
        self.entries = entries;
        self.validity = ReaddirIndexValidity::Valid;
        self.tombstone_count = 0;
    }

    /// Returns the index of the first entry whose cookie is above `cookie`
    /// (spec §4; `partition_point` on the cookie-ordered array).
    fn first_entry_after(&self, cookie: ReaddirCookie) -> Option<usize> {
        let index = self.entries.partition_point(|entry| match entry {
            ReaddirIndexEntry::Visible {
                cookie: entry_cookie,
                ..
            }
            | ReaddirIndexEntry::Tombstone {
                cookie: entry_cookie,
                ..
            } => *entry_cookie <= cookie,
        });
        (index < self.entries.len()).then_some(index)
    }

    /// Converts the `Visible` entry `name` into a `Tombstone` in place
    /// (spec §4, override 4; I4/I7).
    ///
    /// O(n) by-name find (the dominant maintenance cost; an auxiliary
    /// name→index map is a recorded future optimization, NOT implemented —
    /// priors no-premature-optimization). The strong pin becomes a `Weak`
    /// that never keeps the removed object alive (I7); the cookie slot stays
    /// reserved (I4); `tombstone_count` increments; `validity` stays `Valid`.
    /// Eager compaction runs when `tombstone_count >= live_count` (override 4;
    /// amortized O(1) per removal, memory ≤ 2× live). Returns whether an
    /// entry was removed.
    ///
    /// `#[must_use]` (wave-4 repair item 13): the returned verdict is
    /// consumed by every current caller (`readdir_index_remove` falls back
    /// to `NeedsRebuild` on `false`) and must not be silently discarded by
    /// future callers.
    #[must_use]
    pub(super) fn remove_visible(&mut self, name: &str) -> bool {
        let Some(index) = self.entries.iter().position(|entry| {
            matches!(
                entry,
                ReaddirIndexEntry::Visible { name: entry_name, .. } if entry_name == name
            )
        }) else {
            return false;
        };
        let (name, cookie, inode) = match &self.entries[index] {
            ReaddirIndexEntry::Visible {
                name,
                cookie,
                inode,
                ..
            } => (name.clone(), *cookie, inode.clone()),
            _ => return false,
        };
        self.entries[index] = ReaddirIndexEntry::Tombstone {
            name,
            cookie,
            inode: Arc::downgrade(&inode),
        };
        self.tombstone_count += 1;
        // `entries.len() == tombstone_count + live_count` (I4/override 4).
        if self.tombstone_count >= self.entries.len() - self.tombstone_count {
            self.compact_tombstones();
        }
        true
    }

    /// Strict revive-vs-create (spec §4, override 4; I4/I7).
    ///
    /// REVIVE (returns `true`): a `Tombstone` with `name` exists AND its
    /// `Weak::upgrade()` succeeds AND `Arc::ptr_eq(upgraded, &inode)` (the
    /// same logical object) — keep the cookie, convert to `Visible`, restore
    /// the strong pin and `type_`, decrement `tombstone_count`. CREATE
    /// (returns `false`): append a new `Visible` entry at the end with a
    /// fresh cookie from `next_cookie`. The caller (meso 06) must only use
    /// the create path when it can prove the new name's correct visible
    /// position is the end of the cookie order; a mid-sequence insert must
    /// instead mark `NeedsRebuild` (BC-3 §27.1) — never renumber
    /// already-exposed cookies.
    ///
    /// `#[must_use]` (wave-4 repair item 13): the revive-vs-create verdict
    /// is consumed by every current caller (`readdir_index_insert` falls
    /// back to `NeedsRebuild` on the create path) and must not be silently
    /// discarded by future callers.
    #[must_use]
    pub(super) fn insert_visible(
        &mut self,
        name: &str,
        inode: Arc<OverlayInode>,
        type_: InodeType,
    ) -> bool {
        if let Some(index) = self.entries.iter().position(|entry| {
            matches!(
                entry,
                ReaddirIndexEntry::Tombstone { name: entry_name, .. } if entry_name == name
            )
        }) {
            // Clone the revive data first: the tombstone borrow must end
            // before `self.entries[index]` is mutated in place.
            let revive = match &self.entries[index] {
                // The pattern binding is renamed (`weak_inode`) so the
                // `Arc::ptr_eq` below compares the upgraded tombstone against
                // the passed `inode` parameter, not against the `Weak`.
                ReaddirIndexEntry::Tombstone {
                    name,
                    cookie,
                    inode: weak_inode,
                } => match weak_inode.upgrade() {
                    Some(upgraded) if Arc::ptr_eq(&upgraded, &inode) => {
                        Some((name.clone(), *cookie))
                    }
                    _ => None,
                },
                _ => None,
            };
            if let Some((name, cookie)) = revive {
                self.entries[index] = ReaddirIndexEntry::Visible {
                    name,
                    cookie,
                    inode,
                    type_,
                };
                self.tombstone_count -= 1;
                return true;
            }
            // A same-name tombstone that no longer resolves to the same
            // object cannot be revived in place: fall through to the CREATE
            // path with a fresh cookie (I4).
        }
        // CREATE: append at the end with a fresh cookie from the high-water
        // mark (the caller proves the end-of-order position).
        let cookie = self.next_cookie;
        self.next_cookie = ReaddirCookie(self.next_cookie.0.saturating_add(1));
        self.entries.push(ReaddirIndexEntry::Visible {
            name: name.into(),
            cookie,
            inode,
            type_,
        });
        false
    }

    /// Drops all tombstones in place (spec §4; override 4).
    ///
    /// Private, invariant-preserving: O(n) in-place filter under the
    /// already-held index `INODE` lock inside the same `DIR` transaction —
    /// keeps the `Visible` entries (order and cookies untouched), drops ALL
    /// tombstones, resets `tombstone_count`; `validity` stays `Valid`. No
    /// BIO, no source scan: adds no new lock hazard.
    fn compact_tombstones(&mut self) {
        self.entries
            .retain(|entry| matches!(entry, ReaddirIndexEntry::Visible { .. }));
        self.tombstone_count = 0;
    }
}
