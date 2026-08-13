// SPDX-License-Identifier: MPL-2.0

//! The merged-directory readdir index.
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
//! **opaque directory** in a lower layer is a lower-search barrier: the layer's own
//! names still surface, but names in the layers below it never do.
//!
//! ## Lock contract
//!
//! The parent directory transaction lock is taken first, then a brief inode
//! `facts` lock, then the inode `readdir_index` lock; this module acquires no
//! other overlay lock.
//!
//! ## `..` identity
//!
//! The `..` entry carries the resolved overlay-parent identity from
//! [`OverlayInode::resolve_parent_object_id`].

use hashbrown::HashSet;

use super::{
    mount::OverlayFs,
    projection::{OverlayInode, OverlayObjectFacts, PositiveKind, RealObject},
};
use crate::{
    fs::{
        file::InodeType,
        utils::DirentVisitor,
        vfs::{inode::Inode, path::is_dot_or_dotdot},
    },
    prelude::*,
};

/// Cookie value used as the readdir offset cursor.
///
/// This is an ordered scalar domain distinct from a raw `usize` offset:
/// `Ord` supports binary-search `partition_point`, while `Hash`/`Eq` keep the
/// newtype usable as a key/cursor.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct ReaddirCookie(u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ReaddirIndexValidity {
    Valid,
    NeedsRebuild,
}

/// The readdir index for an overlay merged directory; `entries` are ordered by ascending `cookie`.
pub(super) struct ReaddirIndex {
    pub(super) entries: Vec<ReaddirIndexEntry>,
    validity: ReaddirIndexValidity,
    next_cookie: ReaddirCookie,
    tombstone_count: usize,
}

pub(super) enum ReaddirIndexEntry {
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
    pub(in overlayfs) fn readdir_at_impl(
        &self,
        offset: usize,
        visitor: &mut dyn DirentVisitor,
    ) -> Result<usize> {
        let dir = self.dir().ok_or_else(|| Error::new(Errno::ENOTDIR))?;

        let _dir_guard = dir.lock();
        let facts = self.facts_snapshot();
        let input_cookie = ReaddirCookie(offset as u64);
        let facts = self.ensure_readdir_index(&facts)?;
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
        let index = self
            .readdir_index()
            .ok_or_else(|| Error::new(Errno::ENOTDIR))?;
        let index = index.lock();
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
            // `d_ino` derives from the shared identity policy: the child
            // inode's precomputed `object_id` (`Inode::ino()`).
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
    pub(super) fn invalidate_readdir_index(&self) {
        if let Some(index) = self.readdir_index() {
            index.lock().validity = ReaddirIndexValidity::NeedsRebuild;
        }
    }

    /// Inserts a freshly visible name (create/mkdir/mknod/symlink/link
    /// publication) into a `Valid` upper-only index without a full rebuild
    /// because a merged/lower-backed or stale index cannot provably keep
    /// the cookie order.
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
    pub(super) fn readdir_index_remove(&self, name: &str) {
        let Some(index) = self.readdir_index() else {
            return;
        };
        let mut index = index.lock();
        if index.validity == ReaddirIndexValidity::Valid {
            if !index.remove_visible(name) {
                index.validity = ReaddirIndexValidity::NeedsRebuild;
            }
        }
    }

    /// Counts the visible children.
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

    pub(super) fn readdir_index(&self) -> Option<&Mutex<ReaddirIndex>> {
        self.readdir_index.as_ref()
    }

    /// Ensures the directory's index is `Valid`.
    ///
    /// Returns the facts the index was published from; a persistent mismatch
    /// surfaces `EIO` and never publishes a stale index, and a failed scan
    /// leaves the previous `Valid` index intact.
    pub(super) fn ensure_readdir_index(
        &self,
        facts: &OverlayObjectFacts,
    ) -> Result<OverlayObjectFacts> {
        let index = self.readdir_index().ok_or_else(|| {
            Error::with_message(Errno::ENOTDIR, "the overlay inode is not a directory")
        })?;
        {
            let index = index.lock();
            if index.validity == ReaddirIndexValidity::Valid {
                return Ok(facts.clone());
            }
        }
        let mut scan_facts = facts.clone();
        let mut retried = false;
        let sequence = loop {
            let sequence = self.readdir_sequence(&scan_facts)?;
            // Revalidate the facts snapshot after the released-lock segment —
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
                // rebuild; never publish a stale index — leave
                // `NeedsRebuild` and surface the refusal (`EIO` is the tree's
                // consistency-refusal convention).
                return Err(Error::with_message(
                    Errno::EIO,
                    "the overlay directory facts changed while the readdir index was being rebuilt",
                ));
            }
            retried = true;
            scan_facts = revalidated;
        };
        index.lock().rebuild(sequence);
        Ok(scan_facts)
    }

    /// Observes the current visible sequence of this directory from the
    /// pinned layer real objects.
    ///
    /// Scans the upper (when present) and then the lowers top-to-bottom,
    /// stops after the first opaque layer, dedupes by visible name, and
    /// never scans `.`/`..`.
    fn readdir_sequence(
        &self,
        facts: &OverlayObjectFacts,
    ) -> Result<Vec<(String, Arc<OverlayInode>, InodeType)>> {
        let fs = self.fs();
        let fs = fs.downcast_ref::<OverlayFs>().ok_or_else(|| {
            Error::with_message(
                Errno::EIO,
                "the overlay inode is not backed by an overlay mount",
            )
        })?;
        let layers: Vec<&RealObject> = match facts.kind() {
            PositiveKind::Single => {
                let source = match facts.upper() {
                    Some(upper) => upper,
                    // The `upper.is_some() || !lowers.is_empty()` facts
                    // invariant guarantees `lowers[0]`.
                    None => &facts.lowers()[0],
                };
                vec![source]
            }
            PositiveKind::Merged => {
                let mut layers = Vec::new();
                for layer in facts.upper().into_iter().chain(facts.lowers().iter()) {
                    layers.push(layer);
                    if layer.is_opaque_directory()? {
                        break;
                    }
                }
                layers
            }
        };
        let mut seen = HashSet::new();
        let mut sequence = Vec::new();
        for layer in layers {
            for name in self.collect_layer_names(layer)? {
                if !seen.insert(name.clone()) {
                    continue;
                }
                if let Some(inode) = fs.lookup_binding(facts, &name)?.binding.into_inode() {
                    let file_type = inode.type_();
                    sequence.push((name, inode, file_type));
                }
            }
        }
        Ok(sequence)
    }

    fn collect_layer_names(&self, layer: &RealObject) -> Result<Vec<String>> {
        let mut names: Vec<String> = Vec::new();
        layer.real_inode().readdir_at(0, &mut names)?;
        names.retain(|name| !is_dot_or_dotdot(name));
        Ok(names)
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

    /// Rebuilds the index from a complete visible sequence.
    ///
    /// A name that was `Visible` before, points to the same logical object
    /// (`Arc::ptr_eq`), and has its previous cookie above the last cookie
    /// already assigned in this rebuild keeps that cookie. Any other
    /// appearance receives a fresh cookie from the never-decreasing
    /// `next_cookie`.
    ///
    /// The rebuild discards every tombstone and sets `validity` to `Valid`;
    /// `last_assigned` only moves forward, so the resulting cookie order stays
    /// monotonic.
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
