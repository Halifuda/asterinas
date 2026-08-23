// SPDX-License-Identifier: MPL-2.0
#![short_vis_path::add(overlayfs)]

//! The merged-directory readdir index and enumeration service.
//!
//! A merged overlay directory must iterate its visible names in a stable,
//! resumable order, so each overlay directory keeps one [`ReaddirIndex`].
//!
//! ## Index contract
//!
//! The index is the first source for visible names: exactly one current
//! [`ReaddirIndex`] exists per overlay directory (`Some` iff directory);
//! cookies are monotonic and never reused, with `1`/`2` reserved for `.`/`..`.
//!
//! A **`Tombstone`** entry records a deleted name that keeps its cookie. An
//! **opaque directory** in a lower layer is a lower-search barrier: the
//! layer's own names still surface, but names in the layers below it never do.
//!
//! ## `..` identity
//!
//! The `..` entry carries the resolved overlay-parent identity from
//! [`OverlayInode::resolve_parent_object_id`].

use hashbrown::HashSet;

use super::{Lookup, OverlayInode, identity::ObjectId, lookup::is_opaque_directory};
use crate::{
    fs::{
        file::InodeType,
        fs_impls::overlayfs::{fs::OverlayFs, layer::RealObjectStack, real::RealObject},
        utils::DirentVisitor,
        vfs::inode::Inode,
    },
    prelude::*,
};

/// Cookie value used as the readdir offset cursor.
///
/// This is an ordered scalar domain distinct from a raw `usize` offset:
/// `Ord` supports binary-search `partition_point`, while `Hash`/`Eq` keep the
/// newtype usable as a key/cursor.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct ReaddirCookie(u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReaddirIndexValidity {
    Valid,
    NeedsRebuild,
}

/// The readdir index for an overlay merged directory; `entries` are ordered by ascending `cookie`.
pub(super) struct ReaddirIndex {
    entries: Vec<ReaddirIndexEntry>,
    validity: ReaddirIndexValidity,
    next_cookie: ReaddirCookie,
    tombstone_count: usize,
}

enum ReaddirIndexEntry {
    Visible {
        name: String,
        cookie: ReaddirCookie,
        inode: Arc<OverlayInode>,
        type_: InodeType,
    },
    Tombstone {
        name: String,
        cookie: ReaddirCookie,
        inode: Weak<OverlayInode>,
    },
}

impl OverlayInode {
    /// Serves the VFS readdir entry: the synthesized `.`/`..` head entries and the
    /// index's visible real entries in cookie order.
    ///
    /// `offset` selects the next entry after that cookie, and the returned delta
    /// is `last_visited_cookie - offset` (0 when nothing is consumed). `Tombstone`
    /// entries are skipped; a non-directory receiver fails with `ENOTDIR`.
    pub(super) fn readdir_at_impl(
        &self,
        offset: usize,
        visitor: &mut dyn DirentVisitor,
    ) -> Result<usize> {
        let mut lock = self.lock.lock();
        let index = lock.as_mut().ok_or_else(|| Error::new(Errno::ENOTDIR))?;
        let facts = self.real_object_stack();
        let input_cookie = ReaddirCookie(offset as u64);
        let facts = self.ensure_readdir_index(&facts, index)?;
        let mut last_visited: Option<ReaddirCookie> = None;
        let delta_fn = |last_visited: Option<ReaddirCookie>| -> usize {
            let delta = match last_visited {
                Some(last) => last.0 - input_cookie.0,
                None => 0,
            };
            usize::try_from(delta).unwrap_or(usize::MAX)
        };
        if input_cookie < ReaddirCookie(1) {
            visitor.visit(".", self.ino(), InodeType::Dir, 1)?;
            last_visited = Some(ReaddirCookie(1));
        }
        if input_cookie < ReaddirCookie(2) {
            let parent_object_id = self.resolve_parent_object_id(&facts);
            if visitor
                .visit("..", parent_object_id.ino, InodeType::Dir, 2)
                .is_err()
            {
                // `.` was already consumed by this call, so the consumed
                // delta is returned.
                return Ok(delta_fn(last_visited));
            }
            last_visited = Some(ReaddirCookie(2));
        }
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
                continue;
            };
            let d_off = match usize::try_from(cookie.0) {
                Ok(d_off) => d_off,
                Err(_) => break,
            };
            // `d_ino` is the shared identity-policy `object_id`.
            if let Err(err) = visitor.visit(name, inode.ino(), *type_, d_off) {
                if last_visited.is_none() {
                    return Err(err);
                }
                break;
            }
            last_visited = Some(*cookie);
        }
        Ok(delta_fn(last_visited))
    }
}

impl OverlayInode {
    /// Invalidates the index for namespace mutations
    /// and copy-up directory-authority transitions.
    ///
    /// Callers must already hold `self.lock`.
    pub(super) fn invalidate_readdir_index(&self, index: &mut Option<ReaddirIndex>) {
        if let Some(index) = index.as_mut() {
            index.validity = ReaddirIndexValidity::NeedsRebuild;
        }
    }

    /// Inserts a freshly visible name (create/mkdir/mknod/symlink/link
    /// publication) into a `Valid` upper-only index without a full rebuild
    /// because a merged/lower-backed or stale index cannot provably keep
    /// the cookie order.
    ///
    /// Callers must already hold `self.lock`.
    pub(super) fn readdir_index_insert(
        &self,
        name: &str,
        inode: Arc<OverlayInode>,
        type_: InodeType,
        index: &mut Option<ReaddirIndex>,
    ) {
        let Some(index) = index.as_mut() else {
            return;
        };
        if index.validity == ReaddirIndexValidity::Valid
            && self.upper.get().is_some()
            && self.lowers.is_empty()
        {
            if !index.insert_visible(name, inode, type_) {
                index.validity = ReaddirIndexValidity::NeedsRebuild;
            }
        } else {
            index.validity = ReaddirIndexValidity::NeedsRebuild;
        }
    }

    /// Removes a hidden/removed name (unlink/rmdir publication) from a
    /// `Valid` index without a full rebuild. If the name cannot be
    /// tombstoned, the index falls back to `NeedsRebuild`.
    ///
    /// Tombstoning preserves the removed name's cookie, so readdir positions
    /// already exposed remain stable; that is why a failed tombstone cannot
    /// stay `Valid`.
    ///
    /// Callers must already hold `self.lock`.
    pub(super) fn readdir_index_remove(&self, name: &str, index: &mut Option<ReaddirIndex>) {
        let Some(index) = index.as_mut() else {
            return;
        };
        if index.validity == ReaddirIndexValidity::Valid && !index.remove_visible(name) {
            index.validity = ReaddirIndexValidity::NeedsRebuild;
        }
    }

    /// Updates this parent's index after a whiteout publication.
    ///
    /// Remove tombstones the hidden name; rename invalidates the whole index
    /// because the visible sequence is reordered. `name` is `Some` for the
    /// remove-style single-name update and `None` for the rename-style
    /// invalidation.
    pub(super) fn finish_whiteout_index(
        &self,
        name: Option<&str>,
        index: &mut Option<ReaddirIndex>,
    ) {
        match name {
            Some(name) => self.readdir_index_remove(name, index),
            None => self.invalidate_readdir_index(index),
        }
    }

    /// Counts the visible children.
    ///
    /// This method acquires the target directory's own `lock`; callers that
    /// already hold it must use `ensure_readdir_index` directly.
    pub(super) fn visible_child_count(&self) -> Result<usize> {
        let mut lock = self.lock.lock();
        let index = lock.as_mut().ok_or_else(|| {
            Error::with_message(Errno::ENOTDIR, "the overlay inode is not a directory")
        })?;
        let facts = self.real_object_stack();
        self.ensure_readdir_index(&facts, index)?;
        Ok(index
            .entries
            .iter()
            .filter(|entry| matches!(entry, ReaddirIndexEntry::Visible { .. }))
            .count())
    }

    /// Ensures the directory's index is `Valid`.
    ///
    /// Returns the facts the index was published from; a persistent mismatch
    /// surfaces `EIO` and never publishes a stale index, and a failed scan
    /// leaves the previous `Valid` index intact. The caller must already hold
    /// `self.lock` and pass the locked index payload.
    pub(super) fn ensure_readdir_index(
        &self,
        facts: &RealObjectStack,
        index: &mut ReaddirIndex,
    ) -> Result<RealObjectStack> {
        if index.validity == ReaddirIndexValidity::Valid {
            return Ok(facts.clone());
        }
        // Directory copy-up preserves the visible sequence (empty upper +
        // retained lowers), so no post-scan facts revalidation is needed.
        let sequence = self.readdir_sequence(facts)?;
        index.rebuild(sequence);
        Ok(facts.clone())
    }

    /// Observes the current visible sequence of this directory from the
    /// pinned layer real objects.
    ///
    /// Scans the upper (when present) and then the lowers top-to-bottom,
    /// stops after the first opaque layer, dedupes by visible name, and
    /// never scans `.`/`..`.
    fn readdir_sequence(
        &self,
        facts: &RealObjectStack,
    ) -> Result<Vec<(String, Arc<OverlayInode>, InodeType)>> {
        let fs = self.fs();
        let fs = fs.downcast_ref::<OverlayFs>().ok_or_else(|| {
            Error::with_message(
                Errno::EIO,
                "the overlay inode is not backed by an overlay mount",
            )
        })?;
        let layers: Vec<&RealObject> = if !facts.is_merged() {
            let source = match facts.upper.as_ref() {
                Some(upper) => upper,
                // The `upper.is_some() || !lowers.is_empty()` facts
                // invariant guarantees `lowers[0]`.
                None => &facts.lowers[0],
            };
            vec![source]
        } else {
            let mut layers = Vec::new();
            for layer in facts.upper.iter().chain(facts.lowers.iter()) {
                layers.push(layer);
                if is_opaque_directory(layer)? {
                    break;
                }
            }
            layers
        };
        let mut seen = HashSet::new();
        let mut sequence = Vec::new();
        for layer in layers {
            for name in crate::fs::fs_impls::overlayfs::read_child_names(layer.real_inode())? {
                if !seen.insert(name.clone()) {
                    continue;
                }
                if let Lookup::Positive(inode) = fs.lookup(self, &name)? {
                    let file_type = inode.type_();
                    sequence.push((name, inode, file_type));
                }
            }
        }
        Ok(sequence)
    }
}

impl OverlayInode {
    /// Resolves the identity published for this directory's `..` entry.
    ///
    /// Serves the child-source-layer real parent identity (exact when the
    /// overlay parent's visible source is on the same layer, otherwise an
    /// approximation), falling back to the stable `d_ino("..") ==
    /// d_ino(".")` self-parent when no disclosure-safe projection exists.
    fn resolve_parent_object_id(&self, facts: &RealObjectStack) -> ObjectId {
        let fs = match self.fs_arc() {
            Ok(fs) => fs,
            Err(err) => {
                warn!(
                    "overlay readdir: the owning mount is unavailable ({:?}); \
                     falling back to d_ino(\"..\") == d_ino(\".\")",
                    err
                );
                return self.parent_fallback();
            }
        };
        // Overlay-root special case: `..` is the root itself (Unix
        // self-parent); the underlying `lookup("..")` is skipped.
        if self.is_mount_root(&fs) {
            return self.parent_fallback();
        }
        // Determinism short-circuit: on a multi-fs xino-off mount the
        // projection matrix takes the xino-off/overflow directory branch for
        // EVERY parent (a fresh fallback ino per call — unstable), so the
        // whole route is predetermined to serve the stable self-parent
        // approximation; skip the underlying `lookup("..")`/origin read whose
        // result would only be discarded.
        if !fs.identity().is_xino_effective() && !fs.identity().is_all_layers_same_fs() {
            return self.parent_fallback();
        }
        let visible = facts.visible_source();
        let parent_real_inode = match visible.real_inode().lookup("..") {
            Ok(parent) => parent,
            Err(err) => {
                warn!(
                    "overlay readdir: `..` resolution on the visible source failed \
                     ({:?}); falling back to d_ino(\"..\") == d_ino(\".\")",
                    err
                );
                return self.parent_fallback();
            }
        };
        // Upper-backed real parent: prefer the durable lower-id record so the
        // `..` identity matches the parent's record-derived `stat("..")`,
        // gated on deterministic projection.
        if visible.layer_index() == 0
            && let Some(object_id) = self.project_parent_from_lower_record(&fs, &parent_real_inode)
        {
            return object_id;
        }
        if !fs
            .identity()
            .is_directory_projection_deterministic(visible.fsid(), parent_real_inode.ino())
        {
            return self.parent_fallback();
        }
        let parent_real = RealObject::identity_only(
            visible.layer_index(),
            parent_real_inode,
            visible.fsid(),
            visible.container_dev_id(),
        );
        fs.identity().project_object_id(&parent_real, true)
    }

    /// Projects the upper-backed real parent's identity from its durable
    /// origin record, gated on deterministic projection.
    ///
    /// Returns `None` when no readable record resolves to a current lower
    /// layer or the projection would be non-deterministic; the caller then
    /// attempts the visible-source projection. The underlying `read_lower_id`
    /// is caller-credential-gated, so `d_ino("..")` may differ between
    /// privileged and unprivileged readers (logged at `debug!`).
    fn project_parent_from_lower_record(
        &self,
        fs: &OverlayFs,
        parent_real_inode: &Arc<dyn Inode>,
    ) -> Option<ObjectId> {
        match fs.read_lower_id(parent_real_inode) {
            Ok(Some(record)) => {
                // When all layers share one filesystem, projection passes the origin
                // through without a layer id, so this caller skips the layer resolution.
                if !fs.identity().is_all_layers_same_fs() {
                    let layer_id = fs.identity().resolve_layer_id_for_record(
                        record.container_dev_id(),
                        record.lower_layer_root_ino(),
                    )?;
                    if !fs
                        .identity()
                        .is_directory_projection_deterministic(layer_id, record.real_ino())
                    {
                        return None;
                    }
                }
                fs.identity().project_object_id_from_lower_id(&record, true)
            }
            Ok(None) => None,
            Err(err) if matches!(err.error(), Errno::EACCES | Errno::EPERM) => {
                debug!(
                    "overlay readdir: the parent's origin record is \
                     credential-gated ({:?}); d_ino(\"..\") may differ between \
                     privileged and unprivileged readers until the VFS can \
                     read xattrs with the caller's credentials; falling back to the \
                     visible-source projection",
                    err
                );
                None
            }
            Err(err) => {
                debug!(
                    "overlay readdir: the parent's origin record is unreadable \
                     ({:?}); falling back to the visible-source projection",
                    err
                );
                None
            }
        }
    }

    /// Returns whether this inode is the overlay mount root (the self-parent
    /// special case of the `..` route).
    ///
    /// The check looks up the mount root's visible-source key in the inode
    /// cache and compares it against `self.key()`. It fails closed (serves
    /// the self-parent fallback) when the root is not registered, without
    /// disclosing the backing-store parent.
    fn is_mount_root(&self, fs: &OverlayFs) -> bool {
        match fs.inodes().get(fs.root_visible_key()) {
            Some(root) => root.key() == self.key(),
            None => {
                warn!(
                    "overlay readdir: the mount root inode is not registered in the inode cache; \
                     serving the self-parent fallback"
                );
                true
            }
        }
    }

    /// Returns the `d_ino("..") == d_ino(".")` approximation: the stable
    /// fallback identity served when the real parent cannot be resolved
    /// disclosure-safely or deterministically (overlay root, xino-off /
    /// overflow directory branch, unresolvable real parent, or unavailable
    /// owning mount).
    fn parent_fallback(&self) -> ObjectId {
        self.object_id()
    }
}

impl ReaddirIndex {
    /// Constructs the empty initial index.
    ///
    /// Every directory's index is built through this constructor
    /// (`NeedsRebuild` initial state).
    pub(super) fn new() -> Self {
        Self {
            entries: Vec::new(),
            validity: ReaddirIndexValidity::NeedsRebuild,
            next_cookie: ReaddirCookie(3),
            tombstone_count: 0,
        }
    }

    /// Returns the visible inode pins in cookie order, skipping tombstones.
    pub(super) fn visible_inodes(&self) -> Vec<Arc<OverlayInode>> {
        self.entries
            .iter()
            .filter_map(|entry| match entry {
                ReaddirIndexEntry::Visible { inode, .. } => Some(inode.clone()),
                ReaddirIndexEntry::Tombstone { .. } => None,
            })
            .collect()
    }

    /// Returns the still-remembered visible inode for `name`, if any.
    ///
    /// This is the in-overlay per-name record used to detect an upper-backed
    /// name that disappeared behind the overlay.
    pub(super) fn visible_inode(&self, name: &str) -> Option<Arc<OverlayInode>> {
        self.entries.iter().find_map(|entry| match entry {
            ReaddirIndexEntry::Visible {
                name: entry_name,
                inode,
                ..
            } if entry_name == name => Some(inode.clone()),
            _ => None,
        })
    }

    /// Rebuilds the index from a complete visible sequence.
    ///
    /// A name that was `Visible` before, points to the same logical object,
    /// and has its previous cookie above `last_assigned` keeps that cookie;
    /// any other appearance receives a fresh cookie.
    ///
    /// The rebuild discards every tombstone and sets `validity` to `Valid`;
    /// `last_assigned` only moves forward, so cookie order stays monotonic.
    fn rebuild(&mut self, sequence: Vec<(String, Arc<OverlayInode>, InodeType)>) {
        let mut entries = Vec::with_capacity(sequence.len());
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
                    // cookie exhaustion is unreachable for any real directory; saturating keeps the cookie ordering monotonic.
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

    /// Returns the index of the first entry whose cookie is above `cookie`.
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

    /// Converts the `Visible` entry `name` into a `Tombstone` in place (O(n)
    /// by-name find, the dominant maintenance cost).
    #[must_use]
    fn remove_visible(&mut self, name: &str) -> bool {
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
        if self.tombstone_count >= self.entries.len() - self.tombstone_count {
            self.compact_tombstones();
        }
        true
    }

    /// Revives or creates the visible entry.
    ///
    /// The caller must only use the
    /// create path when it can prove the new name's correct visible position
    /// is the end of the cookie order; a mid-sequence insert must instead
    /// mark `NeedsRebuild` — never renumber already-exposed cookies.
    #[must_use]
    fn insert_visible(&mut self, name: &str, inode: Arc<OverlayInode>, type_: InodeType) -> bool {
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
        }
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

    /// Drops all tombstones, retaining only the visible entries.
    fn compact_tombstones(&mut self) {
        self.entries
            .retain(|entry| matches!(entry, ReaddirIndexEntry::Visible { .. }));
        self.tombstone_count = 0;
    }
}
