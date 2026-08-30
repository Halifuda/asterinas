// SPDX-License-Identifier: MPL-2.0
#![short_vis_path::add(overlayfs)]

//! Upper-first layer lookup and inode projection.
//!
//! This module owns the lookup path: it resolves a name upper-first across
//! the layer stack, projects the winning real-object stack into the shared
//! [`OverlayInode`], and returns the simple positive/negative [`Lookup`]
//! result.
//!
//! # Lookup scan
//!
//! The lookup scan is upper-first with overlayfs merge-stop semantics:
//! the first non-directory hit terminates as a single-object result;
//! directory hits accumulate into the lower stack until a barrier — a
//! whiteout, an opaque directory, or a non-directory below an accumulated
//! directory — or the upper-miss opaque-parent case.

use spin::Once;

use crate::{
    fs::{
        file::InodeType,
        fs_impls::overlayfs::{
            fs::OverlayFs,
            inode::{
                OverlayInode, ReaddirIndex,
                xattr::{
                    MarkerReadSemantics, has_marker, opaque_marker_name, whiteout_marker_name,
                },
            },
            layer::RealObjectStack,
            real::RealObject,
        },
        vfs::inode::{Extension, Inode},
    },
    prelude::*,
};

/// The overlay-visible result of resolving one name under a directory.
#[derive(Clone)]
pub(super) enum Lookup {
    Positive(Arc<OverlayInode>),
    Negative(NegativeLookup),
}

/// The reason a name is not visible.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum NegativeLookup {
    /// The name is absent from every layer.
    Absent,
    /// The name is hidden by a whiteout barrier.
    HiddenByWhiteout,
    /// The name is hidden by an opaque-directory barrier.
    HiddenByOpaque,
}

/// How a projected [`OverlayInode`] binds into the namespace: the mount
/// root (its self-parent `Weak` is built by `Arc::new_cyclic`) or a named
/// child under its canonical parent.
#[derive(Clone, Copy)]
pub(super) enum ProjectionBinding<'a> {
    Root,
    Child {
        parent: &'a Arc<OverlayInode>,
        name: &'a str,
    },
}

/// Returns whether `real_inode` is a whiteout.
///
/// `true` when either: the object is a character device with device
/// number `0:0` (backends report it as `Some(DeviceId::null())`, or as
/// `None` for a zero device number such as ramfs); or the
/// `trusted.overlay.whiteout` xattr value is exactly `'y'`. An `ERANGE`,
/// `ENODATA`, or `EOPNOTSUPP` marker read is not a whiteout (`false`);
/// any other error propagates.
pub(super) fn is_whiteout_inode(real_inode: &Arc<dyn Inode>) -> Result<bool> {
    let metadata = real_inode.metadata()?;
    if metadata.type_ == InodeType::CharDevice
        && metadata.self_dev_id.is_none_or(|dev_id| dev_id.is_null())
    {
        return Ok(true);
    }
    has_marker(
        real_inode,
        whiteout_marker_name()?,
        MarkerReadSemantics::ValueY,
    )
}

/// Returns whether `real` is an opaque directory (a lower-search barrier).
///
/// A non-directory is never opaque (`false`). A directory is opaque exactly
/// when the `trusted.overlay.opaque` xattr value is `'y'`; an `ENODATA`,
/// `EOPNOTSUPP`, or `ERANGE` read is not opaque (`false`), and any other
/// error propagates.
pub(super) fn is_opaque_directory(real: &RealObject) -> Result<bool> {
    if !real.real_inode().type_().is_directory() {
        return Ok(false);
    }
    has_marker(
        real.real_inode(),
        opaque_marker_name()?,
        MarkerReadSemantics::ValueY,
    )
}

impl OverlayFs {
    /// Runs the upper-first layer lookup for `name` inside `parent`'s
    /// real layers, with overlayfs merge-stop semantics.
    fn lookup_in_layers(&self, parent: &Arc<OverlayInode>, name: &str) -> Result<Lookup> {
        let mut dir_hits: Vec<RealObject> = Vec::new();

        if let Some(upper_real) = parent.upper.get() {
            let upper_path = upper_real.real_path()?;
            match crate::fs::fs_impls::overlayfs::lookup_child_path(&upper_path, name) {
                Ok(child_path) => {
                    let hit = RealObject::child_hit(0, &child_path, upper_real);
                    if is_whiteout_inode(hit.real_inode())? {
                        return Ok(Lookup::Negative(NegativeLookup::HiddenByWhiteout));
                    }
                    if !hit.real_inode().type_().is_directory() {
                        return Ok(Lookup::Positive(self.project_inode(
                            &RealObjectStack::upper_only(hit),
                            ProjectionBinding::Child { parent, name },
                        )));
                    }
                    if is_opaque_directory(&hit)? {
                        return Ok(Lookup::Positive(self.project_inode(
                            &RealObjectStack::upper_only(hit),
                            ProjectionBinding::Child { parent, name },
                        )));
                    }
                    dir_hits.push(hit);
                }
                Err(err) if err.error() == Errno::ENOENT => {
                    if is_opaque_directory(upper_real)? {
                        return Ok(Lookup::Negative(NegativeLookup::HiddenByOpaque));
                    }
                }
                Err(err) => return Err(err),
            }
        }

        for lower_real in &parent.lowers {
            let layer_index = lower_real.layer_index();
            let lower_path = lower_real.real_path()?;
            match crate::fs::fs_impls::overlayfs::lookup_child_path(&lower_path, name) {
                Ok(child_path) => {
                    let hit = RealObject::child_hit(layer_index, &child_path, lower_real);
                    if is_whiteout_inode(hit.real_inode())? {
                        // A whiteout is the topmost occurrence of the name:
                        // the name is hidden. Below an already-visible
                        // directory it only ends the downward merge scan.
                        if dir_hits.is_empty() {
                            return Ok(Lookup::Negative(NegativeLookup::HiddenByWhiteout));
                        }
                        break;
                    }
                    if !hit.real_inode().type_().is_directory() {
                        if dir_hits.is_empty() {
                            return Ok(Lookup::Positive(self.project_inode(
                                &RealObjectStack::lower_only(hit),
                                ProjectionBinding::Child { parent, name },
                            )));
                        }
                        // A non-directory below an accumulated directory hit
                        // stops the downward merge: every deeper layer stays
                        // hidden.
                        break;
                    }
                    let is_opaque = is_opaque_directory(&hit)?;
                    dir_hits.push(hit);
                    if is_opaque {
                        break;
                    }
                }
                Err(err) if err.error() == Errno::ENOENT => continue,
                Err(err) => return Err(err),
            }
        }

        if dir_hits.is_empty() {
            return Ok(Lookup::Negative(NegativeLookup::Absent));
        }

        let upper = if dir_hits[0].layer_index() == 0 {
            Some(dir_hits.remove(0))
        } else {
            None
        };
        Ok(Lookup::Positive(self.project_inode(
            &RealObjectStack::new(upper, dir_hits),
            ProjectionBinding::Child { parent, name },
        )))
    }

    /// Resolves one `name` under `parent` into a [`Lookup`].
    ///
    /// The flow is pure resolve: the layer-ordered lookup re-observes fresh
    /// layer truth and projects it directly, with no verify-then-serve cache.
    pub(super) fn lookup(&self, parent: &OverlayInode, name: &str) -> Result<Lookup> {
        let parent_arc = self.inodes().get(parent.key()).ok_or_else(|| {
            Error::with_message(
                Errno::EIO,
                "the overlay parent is not registered under its visible-source key",
            )
        })?;
        self.lookup_in_layers(&parent_arc, name)
    }

    /// Creates or reuses the shared [`OverlayInode`] for `facts`.
    ///
    /// The `object_id` is computed lazily: a valid cache hit is returned
    /// without reading the upper's lower-id origin record. On a miss the
    /// lower-id read is still done before the inode-cache write path, so it
    /// never runs inside the cache's upgraded guard.
    ///
    /// A validated hit returns the existing inode unwritten; the binding
    /// shapes only the miss-side construction.
    pub(super) fn project_inode(
        &self,
        facts: &RealObjectStack,
        binding: ProjectionBinding<'_>,
    ) -> Arc<OverlayInode> {
        let source = facts.visible_source();
        let key = facts.key();
        let is_directory = facts.is_merged() || source.real_inode().type_().is_directory();
        // Clone the visible source before the closures move `facts`: the
        // get-or-create predicate validates a cached hit against this real
        // inode, replacing an ino-reuse stale occupant. The fresh-truth
        // upper presence is captured here as well, because the predicate
        // must distinguish a lower-only fresh truth (below) from an
        // upper-backed one.
        let source_inode = facts.visible_source().real_inode().clone();
        let fresh_is_lower_only = facts.upper.is_none();
        if let Some(inode) = self.inodes().get(key) {
            let hit_valid = if fresh_is_lower_only {
                // Reuse only an inode whose visible source is exactly
                // this lower; a stale-upper inode must not be reused even
                // though `contains_real_inode` matches the retained
                // lower, because its dead-upper metadata would be wrong.
                Arc::ptr_eq(inode.visible_source().real_inode(), &source_inode)
            } else {
                inode.contains_real_inode(&source_inode)
            };
            if hit_valid {
                return inode;
            }
        }
        let fallback_fn = || self.identity().project_object_id(source, is_directory);
        let object_id = if source.layer_index() == 0 {
            match self.read_lower_id(source.real_inode()) {
                // Defensive: the record was device-validated at the read boundary,
                // so `None` here is the absent/ambiguous-device corner.
                Ok(Some(record)) => {
                    // The record is accepted only when its real inode is
                    // consistent with the retained same-layer lower of the
                    // fresh facts.
                    if self.identity().origin_real_ino_resolves(&record, facts) {
                        self.identity()
                            .project_object_id_from_lower_id(&record, is_directory)
                            .unwrap_or_else(fallback_fn)
                    } else {
                        fallback_fn()
                    }
                }
                Ok(None) => fallback_fn(),
                Err(err) => {
                    warn!(
                        "failed to read the lower-id record of the upper source; \
                         falling back to the visible-source projection: {:?}",
                        err
                    );
                    fallback_fn()
                }
            }
        } else {
            fallback_fn()
        };
        let fs = self.self_weak().clone();
        let lowers = facts.lowers.clone();
        let upper = facts.upper.clone();
        self.inodes().get_or_create(
            key,
            move |carrier| {
                if fresh_is_lower_only {
                    // Reuse only an inode whose visible source is exactly
                    // this lower; a stale-upper inode must not be reused even
                    // though `contains_real_inode` matches the retained
                    // lower, because its dead-upper metadata would be wrong.
                    Arc::ptr_eq(carrier.visible_source().real_inode(), &source_inode)
                } else {
                    carrier.contains_real_inode(&source_inode)
                }
            },
            move || {
                let upper = match upper {
                    Some(upper) => Once::initialized(upper),
                    None => Once::new(),
                };
                let lock = Mutex::new(if is_directory {
                    Some(ReaddirIndex::new())
                } else {
                    None
                });
                match binding {
                    ProjectionBinding::Root => Arc::new_cyclic(|weak| OverlayInode {
                        fs,
                        lowers,
                        upper,
                        object_id,
                        lock,
                        recorded_parent: RwMutex::new(weak.clone()),
                        copyup: Mutex::new(None),
                        extension: Extension::new(),
                    }),
                    ProjectionBinding::Child { parent, name } => Arc::new(OverlayInode {
                        fs,
                        lowers,
                        upper,
                        object_id,
                        lock,
                        recorded_parent: RwMutex::new(Arc::downgrade(parent)),
                        copyup: Mutex::new(if facts.upper.is_some() {
                            None
                        } else {
                            Some(String::from(name))
                        }),
                        extension: Extension::new(),
                    }),
                }
            },
        )
    }
}
