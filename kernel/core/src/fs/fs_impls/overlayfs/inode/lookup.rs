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

use super::binding_cache::{
    Binding, BindingKey, HiddenEvidence, LayerLookup, LookupOutcome, NegativeBinding,
    PositiveBinding,
};
use crate::{
    fs::{
        file::InodeType,
        fs_impls::overlayfs::{
            fs::OverlayFs,
            inode::{
                OverlayInode,
                xattr::{MarkerReadSemantics, XattrPolicy},
            },
            layer::RealObjectStack,
            real::{RealObject, RealObjectKey},
        },
        vfs::inode::{Extension, Inode},
    },
    prelude::*,
};

/// Returns whether `real_inode` is a whiteout.
///
/// `true` when either: the object is a character device with device
/// number `0:0` (backends report it as `Some(DeviceId::null())`, or as
/// `None` for a zero device number such as ramfs); or the
/// `trusted.overlay.whiteout` xattr value is exactly `'y'`. An `ERANGE`,
/// `ENODATA`, or `EOPNOTSUPP` marker read is not a whiteout (`false`);
/// any other error propagates.
pub(in overlayfs) fn is_whiteout_inode(real_inode: &Arc<dyn Inode>) -> Result<bool> {
    let metadata = real_inode.metadata()?;
    if metadata.type_ == InodeType::CharDevice
        && metadata.self_dev_id.is_none_or(|dev_id| dev_id.is_null())
    {
        return Ok(true);
    }
    XattrPolicy::has_marker(
        &XattrPolicy,
        real_inode,
        XattrPolicy::whiteout_marker_name()?,
        MarkerReadSemantics::ValueY,
    )
}

/// Returns whether `real` is an opaque directory (a lower-search barrier).
///
/// A non-directory is never opaque (`false`). A directory is opaque exactly
/// when the `trusted.overlay.opaque` xattr value is `'y'`; an `ENODATA`,
/// `EOPNOTSUPP`, or `ERANGE` read is not opaque (`false`), and any other
/// error propagates.
pub(in overlayfs) fn is_opaque_directory(real: &RealObject) -> Result<bool> {
    if !real.real_inode().type_().is_directory() {
        return Ok(false);
    }
    XattrPolicy::has_marker(
        &XattrPolicy,
        real.real_inode(),
        XattrPolicy::opaque_marker_name()?,
        MarkerReadSemantics::ValueY,
    )
}

impl OverlayFs {
    /// Runs the upper-first layer lookup for `name` inside `parent_facts`'s
    /// real layers, with overlayfs merge-stop semantics.
    pub(super) fn lookup_in_layers(
        &self,
        parent_facts: &RealObjectStack,
        name: &str,
    ) -> Result<LayerLookup> {
        let mut dir_hits: Vec<RealObject> = Vec::new();

        if let Some(upper_real) = &parent_facts.upper {
            let upper_path = upper_real.real_path()?;
            match crate::fs::fs_impls::overlayfs::lookup_child_path(&upper_path, name) {
                Ok(child_path) => {
                    let hit = RealObject::child_hit(0, &child_path, upper_real);
                    if is_whiteout_inode(hit.real_inode())? {
                        return Ok(LayerLookup::Negative(NegativeBinding::HiddenByWhiteout(
                            HiddenEvidence::new(0, hit.real_inode().clone()),
                        )));
                    }
                    if !hit.real_inode().type_().is_directory() {
                        return Ok(LayerLookup::Positive(RealObjectStack {
                            upper: Some(hit),
                            lowers: Vec::new(),
                        }));
                    }
                    if is_opaque_directory(&hit)? {
                        return Ok(LayerLookup::Positive(RealObjectStack {
                            upper: Some(hit),
                            lowers: Vec::new(),
                        }));
                    }
                    dir_hits.push(hit);
                }
                Err(err) if err.error() == Errno::ENOENT => {
                    if is_opaque_directory(upper_real)? {
                        return Ok(LayerLookup::Negative(NegativeBinding::HiddenByOpaque(
                            HiddenEvidence::new(0, upper_real.real_inode().clone()),
                        )));
                    }
                }
                Err(err) => return Err(err),
            }
        }

        for lower_real in &parent_facts.lowers {
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
                            return Ok(LayerLookup::Negative(NegativeBinding::HiddenByWhiteout(
                                HiddenEvidence::new(layer_index, hit.real_inode().clone()),
                            )));
                        }
                        break;
                    }
                    if !hit.real_inode().type_().is_directory() {
                        if dir_hits.is_empty() {
                            return Ok(LayerLookup::Positive(RealObjectStack {
                                upper: None,
                                lowers: vec![hit],
                            }));
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
            return Ok(LayerLookup::Negative(NegativeBinding::Absent));
        }

        let upper = if dir_hits[0].layer_index() == 0 {
            Some(dir_hits.remove(0))
        } else {
            None
        };
        Ok(LayerLookup::Positive(RealObjectStack {
            upper,
            lowers: dir_hits,
        }))
    }

    /// Resolves one `name` under `parent_facts` into a [`LookupOutcome`].
    ///
    /// The flow is verify-then-serve: the layer-ordered lookup re-observes
    /// the fresh layer truth, and a cached binding is served only when it
    /// matches that truth; otherwise the binding is rebuilt and published.
    pub(super) fn lookup_binding(
        &self,
        parent_facts: &RealObjectStack,
        name: &str,
    ) -> Result<LookupOutcome> {
        let parent_id = parent_facts.key();
        let truth = self.lookup_in_layers(parent_facts, name)?;
        let is_stale_upper = if let Some(binding) = self.bindings.get(&parent_id, name) {
            if binding.matches_truth(&truth) {
                return Ok(LookupOutcome {
                    binding: binding.as_ref().clone(),
                    is_stale_upper: false,
                });
            }
            binding.is_stale_upper(&truth)
        } else {
            false
        };
        let binding = match truth {
            LayerLookup::Positive(facts) => {
                let inode = self.project_inode(&facts);
                Binding::Positive(PositiveBinding::new(inode))
            }
            LayerLookup::Negative(negative) => Binding::Negative(negative),
        };
        if let Binding::Positive(positive) = &binding {
            if let Some(parent) = self.inodes.get(parent_id) {
                positive.inode.try_record_copyup_transition(parent, name);
            } else {
                debug_assert!(
                    false,
                    "a live overlay parent is always registered under its current visible-source key"
                );
                error!(
                    "overlay parent identity inconsistency: no inode-cache entry for \
                     visible-source key {:?}",
                    parent_id
                );
            }
        }
        self.publish_binding(&parent_id, name, binding.clone());
        Ok(LookupOutcome {
            binding,
            is_stale_upper,
        })
    }

    /// Publishes `binding` for `(parent_id, name)` into the binding cache.
    pub(super) fn publish_binding(&self, parent_id: &RealObjectKey, name: &str, binding: Binding) {
        self.bindings.insert(
            BindingKey::new(*parent_id, String::from(name)),
            Arc::new(binding),
        );
    }

    /// Creates or reuses the shared [`OverlayInode`] for `facts`.
    ///
    /// The `object_id` is precomputed from [`IdentityPolicy`] before the
    /// inode-cache check-and-create, because the upper-source lower-id read
    /// may block on the underlying xattr and must never run inside the
    /// cache's upgraded guard.
    pub(super) fn project_inode(&self, facts: &RealObjectStack) -> Arc<OverlayInode> {
        let source = facts.visible_source();
        let key = facts.key();
        let is_directory = facts.is_merged() || source.real_inode().type_().is_directory();
        let fallback_fn = || self.identity.project_object_id(source, is_directory);
        let object_id = if source.layer_index() == 0 {
            match self.read_lower_id(source.real_inode()) {
                // Defensive: the record was device-validated at the read boundary,
                // so `None` here is the absent/ambiguous-device corner.
                Ok(Some(record)) => {
                    // The record is accepted only when its real inode is
                    // consistent with the retained same-layer lower of the
                    // fresh facts.
                    if self.identity.origin_real_ino_resolves(&record, facts) {
                        self.identity
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
        // Clone the visible source before the closures move `facts`: the
        // get-or-create predicate validates a cached hit against this real
        // inode, replacing an ino-reuse stale occupant. The fresh-truth
        // upper presence is captured here as well, because the predicate
        // must distinguish a lower-only fresh truth (below) from an
        // upper-backed one.
        let source_inode = facts.visible_source().real_inode().clone();
        let fresh_is_lower_only = facts.upper.is_none();
        let fs = self.self_weak.clone();
        let facts = facts.clone();
        self.inodes.get_or_create(
            key,
            move |carrier| {
                if fresh_is_lower_only {
                    // Reuse only an inode whose visible source is exactly
                    // this lower; a stale-upper inode must not be reused even
                    // though `contains_real_inode` matches the retained
                    // lower, because its dead-upper metadata would be wrong.
                    Arc::ptr_eq(
                        carrier.facts_snapshot().visible_source().real_inode(),
                        &source_inode,
                    )
                } else {
                    carrier.facts_snapshot().contains_real_inode(&source_inode)
                }
            },
            move || {
                Arc::new(OverlayInode {
                    fs,
                    key: Mutex::new(key),
                    facts: Mutex::new(facts),
                    dir_transaction_lock: if is_directory {
                        Some(Mutex::new(()))
                    } else {
                        None
                    },
                    object_id,
                    extension: Extension::new(),
                    readdir_index: if is_directory {
                        Some(Mutex::new(
                            crate::fs::fs_impls::overlayfs::inode::readdir::ReaddirIndex::new(),
                        ))
                    } else {
                        None
                    },
                    copyup_transition: Mutex::new(None),
                })
            },
        )
    }
}
