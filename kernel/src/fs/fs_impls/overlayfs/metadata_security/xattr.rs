// SPDX-License-Identifier: MPL-2.0

//! The xattr classification policy and delegation entries of the
//! `metadata_security` meso (meso-05; `P1-33` xattr get/set/list delegation).
//!
//! This module hosts the frozen meso-05 spec §4
//! (`metadata_security/xattr.rs`) surface: the payload-less
//! [`OverlayXattrPolicy`] carrier (stateless this wave; owned once by the
//! Wave-3 `OverlayFs::xattr_policy` field, initialized in `mount/build.rs`),
//! its [`XattrClass`] classification result, the module-private known
//! private-name table and prefix constants, the `classify`/`is_private`/
//! `filter_private_names` methods, and the four `Inode`-trait xattr entries
//! (`get_xattr`/`set_xattr`/`list_xattr`/`remove_xattr`).
//!
//! Classification semantics (frozen, BC-5 §50.1): a name under the
//! `trusted.overlay.`/`user.overlay.` private namespace is `Private` when its
//! suffix is a known Overlay record (the `OVERLAY_PRIVATE_SUFFIXES` table)
//! and `Reserved` otherwise (an `overlay.*`-family name is policy-refused and
//! never auto-promoted to `Public`); a `overlay.overlay.` nesting-prefixed
//! name is `Escaped` (`P2-14` seam — refused/filtered this wave, never
//! un-escaped); everything else is `Public` and delegates to the real
//! authority. `is_private` is the judgment method and the meso-04 §7
//! supersession seam: it returns `true` exactly for the `Private`/`Escaped`/
//! `Reserved` classes — the same name set the meso-04 copy-time predicate
//! `copyup/promote.rs::is_overlay_private_xattr_name` excluded — so copy
//! behavior is preserved while the classification authority moves here.
//!
//! Entry contract (spec §2 Case 4/6, §4 classification-ordering note): the
//! classification stage runs **before** `check_permission` for
//! `set_xattr`/`remove_xattr` so a non-`Public` name is refused with no
//! promotion side effect (`ENODATA` for `get_xattr`; `EPERM` for
//! `set_xattr`/`remove_xattr`); `list_xattr` streams the underlying raw name
//! list through [`OverlayXattrPolicy::filter_private_names`] so no private
//! record ever reaches the caller. `get_xattr`/`list_xattr` carry the frozen
//! empty permission demand (`AccessType::ReadOnly`, `Permission::empty()` —
//! spec §8 item 6); `set_xattr`/`remove_xattr` use the uniform mutating shape
//! (`AccessType::Mutating`, `Permission::MAY_WRITE`) with the copy-up inside
//! the real permission stage, then forward under the creator-credential scope
//! through the single private delegation helper `delegate_to_real` (defined
//! in `mod.rs` so the three sibling files share it).
//!
//! Lock contract (spec §3): this module acquires no Overlay lock. The
//! classification stage and the admission surface are lock-free local stages;
//! the only lock progression is inside the meso-04 authority seam
//! (`ensure_upper_authority`, consumed between the two permission stages), and
//! no Overlay lock is ever held across an underlying xattr callback. The
//! underlying xattr ops self-evaluate under the creator-credential scope
//! (ext2/ramfs evidence, spec §4.0), so the explicit real stage is a benign
//! double evaluation kept for security-gate independence.

use crate::{
    fs::{
        file::Permission,
        fs_impls::overlayfs::{AccessType, projection::OverlayInode},
        vfs::{
            inode::Inode,
            xattr::{XATTR_LIST_MAX_LEN, XattrName, XattrNamespace, XattrSetFlags},
        },
    },
    prelude::*,
};

/// The frozen public/private/escaped classification seam (`P1-33`).
///
/// Stateless in this wave: the private-namespace prefixes
/// (`TRUSTED_OVERLAY_PREFIX`, `USER_OVERLAY_PREFIX`) and the escape prefix
/// (`ESCAPED_OVERLAY_PREFIX`) are module-private consts; `P2-13` (userxattr
/// namespace selection) and `P2-14` (escaping) are insertion points that add
/// state here later — no field is pre-baked. Owner/guard: immutable; owned
/// once by `OverlayFs::xattr_policy`; no lock. Declared at the overlayfs
/// ceiling (`pub(in crate::fs::fs_impls::overlayfs)`) because the Wave-3
/// `OverlayFs::xattr_policy` field in `mount/superblock.rs` and its
/// construction in `mount/build.rs` name this type from sibling module trees
/// (the packet's cross-module-reachability override, Wave-3 precedent).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::fs::fs_impls::overlayfs) struct OverlayXattrPolicy;

/// The payload-less four-way classification result of an xattr full name.
///
/// Payload-less by user ruling (revision 01, §9A call): the four-way
/// classification is the frozen semantic (BC-5 §50.1) and every entry
/// branches only `Public`-vs-rest; the per-record owner dispatch of the
/// removed `OverlayPrivateXattr` payload enum is preserved as the
/// module-private `OVERLAY_PRIVATE_SUFFIXES` table plus the owner-dispatch
/// comment below, because no consumer reads the payload this wave (all
/// dispatch targets are insertion points or other Mesos' seams).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::fs::fs_impls::overlayfs) enum XattrClass {
    /// A user.*/system.*/security.*/trusted.* (non-overlay) name: delegate to
    /// the real authority.
    Public,
    /// A known Overlay-private record (suffix in `OVERLAY_PRIVATE_SUFFIXES`);
    /// owner-dispatched; filtered from listing; refused through the generic
    /// path.
    Private,
    /// A `overlay.overlay.` nesting-prefixed name (`P2-14` seam; refused and
    /// filtered this wave).
    Escaped,
    /// An `overlay.*`-family name not in the known private table:
    /// policy-refused, never auto-promoted to `Public` (BC-5 §50.1).
    Reserved,
}

/// Known Overlay-private record suffixes (FILESYSTEM_SPEC_INDEX §2 + BC-5
/// §50). Owner dispatch (comment/insertion-point table — never a pre-baked
/// enum payload):
///   whiteout, opaque -> namespace-mutation owner (meso-06) [insertion point]
///   redirect         -> P2-02 (deferred)
///   origin, upper    -> association/identity work (meso-07) [insertion point]
///   impure           -> directory-index owner (meso-03)
///   nlink            -> copy-up hardlink bookkeeping
///   uuid             -> meso-01 P2-11 persist
///   metacopy         -> P3-03 (deferred)
///   protattr         -> P2-06 fileattr seam (deferred)
const OVERLAY_PRIVATE_SUFFIXES: &[&str] = &[
    "opaque", "whiteout", "redirect", "origin", "impure", "nlink", "upper", "uuid", "metacopy",
    "protattr",
];

/// The private-namespace prefix of the persisted overlay records.
const TRUSTED_OVERLAY_PREFIX: &str = "trusted.overlay.";

/// The user-namespace mirror of the persisted overlay records (`P2-13` seam).
const USER_OVERLAY_PREFIX: &str = "user.overlay.";

/// The one-level nesting-escape prefix of a lower-overlay name (`P2-14` seam).
const ESCAPED_OVERLAY_PREFIX: &str = "overlay.overlay.";

/// The xattr full name of the opaque-directory marker (Linux `OVL_XATTR_OPAQUE`).
///
/// Wave-4 repair item 11: this module already centralizes the overlay-private
/// name knowledge (`OVERLAY_PRIVATE_SUFFIXES` lists the `opaque` suffix), so
/// the name/value pair now lives here as the single declaration; the
/// `dir/create.rs` and `dir/remove.rs` recipes reference these constants
/// instead of redeclaring them (the third copy in `projection/entry.rs` from
/// an earlier wave is outside this packet's write-set and is recorded as
/// remaining debt).
pub(in crate::fs::fs_impls::overlayfs) const OPAQUE_XATTR_FULL_NAME: &str =
    "trusted.overlay.opaque";

/// The opaque marker value (Linux writes `"y"`; the meso-02 reader requires
/// the first byte `b'y'`).
pub(in crate::fs::fs_impls::overlayfs) const OPAQUE_MARKER_VALUE: &[u8] = b"y";

impl OverlayXattrPolicy {
    /// Classifies an xattr full name into the four-way
    /// `Public`/`Private`/`Escaped`/`Reserved` classes (frozen, BC-5 §50.1).
    ///
    /// Pure and lock-free: a name under the private-namespace prefixes whose
    /// suffix is in `OVERLAY_PRIVATE_SUFFIXES` is `Private` and any other
    /// `overlay.*`-family name is `Reserved` (never auto-promoted to
    /// `Public`); a `overlay.overlay.` nesting-prefixed name is `Escaped`;
    /// everything else is `Public` and delegates to the real authority.
    /// Published at the overlayfs ceiling: `classify` is part of the frozen
    /// public/private/escaped classification seam (spec §1 item 4) that
    /// sibling Mesos consume through `OverlayFs::xattr_policy()`.
    pub(in crate::fs::fs_impls::overlayfs) fn classify(&self, full_name: &str) -> XattrClass {
        if let Some(suffix) = full_name
            .strip_prefix(TRUSTED_OVERLAY_PREFIX)
            .or_else(|| full_name.strip_prefix(USER_OVERLAY_PREFIX))
        {
            if OVERLAY_PRIVATE_SUFFIXES.contains(&suffix) {
                XattrClass::Private
            } else {
                XattrClass::Reserved
            }
        } else if full_name.starts_with(ESCAPED_OVERLAY_PREFIX) {
            XattrClass::Escaped
        } else {
            XattrClass::Public
        }
    }

    /// Returns whether `full_name` is an overlay-private xattr name.
    ///
    /// The judgment method: `!matches!(self.classify(full_name),
    /// XattrClass::Public)` — `true` exactly for the `Private`/`Escaped`/
    /// `Reserved` classes, the same name set the meso-04 copy-time predicate
    /// excluded. This is the meso-04 §7 supersession seam: the copy-time
    /// boundary filter that replaces
    /// `copyup/promote.rs::is_overlay_private_xattr_name` (the meso-04 local
    /// predicate is to be replaced by this seam call; no duplicated predicate
    /// survives).
    pub(in crate::fs::fs_impls::overlayfs) fn is_private(&self, full_name: &str) -> bool {
        !matches!(self.classify(full_name), XattrClass::Public)
    }

    /// Streams the null-terminated raw name list from the underlying listing,
    /// skipping every name with `is_private == true` and writing the
    /// survivors to `list_writer`.
    ///
    /// Returns the number of bytes written (each survivor is written with its
    /// trailing null byte). The intermediate raw list is bounded by
    /// `XATTR_LIST_MAX_LEN` (spec §4): the underlying list always fits, so an
    /// oversized real list surfaces as the underlying `ERANGE` before any
    /// survivor is written. Invariant-preserving filter: a private record
    /// (`Private`/`Escaped`/`Reserved`) never leaks through the listing
    /// (BC-5 §50.2). A non-UTF-8 name cannot be an Overlay-private record
    /// (all private names are ASCII), so it is forwarded unchanged rather
    /// than failing or leaking. Private to the `metadata_security` tree (the
    /// streaming pass is spec §1 "Must Remain Internal"); only the `list_xattr`
    /// entry consumes it.
    pub(super) fn filter_private_names(
        &self,
        raw_list: &[u8],
        list_writer: &mut VmWriter,
    ) -> Result<usize> {
        let mut bytes_written = 0;
        for name_bytes in raw_list.split(|&byte| byte == 0) {
            if name_bytes.is_empty() {
                continue;
            }
            let is_private = core::str::from_utf8(name_bytes)
                .is_ok_and(|name| self.is_private(name));
            if is_private {
                continue;
            }
            list_writer.write_fallible(&mut VmReader::from(name_bytes))?;
            list_writer.write_val(&0u8)?;
            bytes_written += name_bytes.len() + 1;
        }
        Ok(bytes_written)
    }
}

impl Inode for OverlayInode {
    // P1-33 xattr get: classification refusal runs first (before the
    // admission, so no authority side effect ever starts for a private name —
    // the classification ordering note, spec §4), then the frozen empty
    // permission demand (`AccessType::ReadOnly`, `Permission::empty()`;
    // namespace gating already ran in the syscall layer, spec §8 item 6),
    // then a creator-credential forward to the current real authority. The
    // underlying `get_xattr` self-evaluates under the creator-credential
    // scope (ext2/ramfs evidence, spec §4.0); the explicit real stage inside
    // `check_permission` is the benign double evaluation kept for
    // gate-independence.
    fn get_xattr(&self, name: XattrName, value_writer: &mut VmWriter) -> Result<usize> {
        if !matches!(
            self.fs_arc()?.xattr_policy().classify(name.full_name()),
            XattrClass::Public
        ) {
            return Err(Error::with_message(
                Errno::ENODATA,
                "the overlay-private xattr is not exposed through the generic get path",
            ));
        }
        self.check_permission(AccessType::ReadOnly, Permission::empty())?;
        self.delegate_to_real(|real| real.get_xattr(name, value_writer))
    }

    // P1-33 xattr set: the classification stage runs BEFORE the mutating
    // admission so a non-`Public` name is refused with no promotion side
    // effect (spec §4 classification-ordering note; BC-5 §49.1), then the
    // uniform mutating shape (`AccessType::Mutating`, `Permission::MAY_WRITE`
    // — the EROFS gate and the copy-up live inside the real stage), then a
    // creator-credential forward. The underlying `set_xattr` self-evaluates
    // under the creator-credential scope (spec §4.0); the explicit real stage
    // is the benign double evaluation.
    fn set_xattr(
        &self,
        name: XattrName,
        value_reader: &mut VmReader,
        flags: XattrSetFlags,
    ) -> Result<()> {
        if !matches!(
            self.fs_arc()?.xattr_policy().classify(name.full_name()),
            XattrClass::Public
        ) {
            return Err(Error::with_message(
                Errno::EPERM,
                "overlay-private records cannot be forged through the generic set path",
            ));
        }
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        self.delegate_to_real(|real| real.set_xattr(name, value_reader, flags))
    }

    // P1-33 xattr list: the frozen empty permission demand (no mode-DAC
    // demand — spec §8 item 6), then the real listing into the bounded
    // `XATTR_LIST_MAX_LEN` intermediate, then the private-name filter
    // streaming pass so `Private`/`Escaped`/`Reserved` records never reach
    // the caller (invariant, BC-5 §50.2). The filtered length returned by
    // `filter_private_names` is the number of bytes written to `list_writer`.
    fn list_xattr(&self, namespace: XattrNamespace, list_writer: &mut VmWriter) -> Result<usize> {
        self.check_permission(AccessType::ReadOnly, Permission::empty())?;
        self.delegate_to_real(|real| {
            let mut raw_list = vec![0u8; XATTR_LIST_MAX_LEN];
            let mut raw_writer = VmWriter::from(&mut raw_list[..]).to_fallible();
            let list_len = real.list_xattr(namespace, &mut raw_writer)?;
            let fs = self.fs_arc()?;
            fs.xattr_policy()
                .filter_private_names(&raw_list[..list_len], list_writer)
        })
    }

    // P1-33 xattr remove: identical shape to `set_xattr` — classification
    // refusal (`EPERM`) before the mutating admission, so a non-`Public` name
    // is refused with no promotion side effect, then the uniform mutating
    // shape and a creator-credential forward to the current real authority.
    fn remove_xattr(&self, name: XattrName) -> Result<()> {
        if !matches!(
            self.fs_arc()?.xattr_policy().classify(name.full_name()),
            XattrClass::Public
        ) {
            return Err(Error::with_message(
                Errno::EPERM,
                "overlay-private records cannot be removed through the generic path",
            ));
        }
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        self.delegate_to_real(|real| real.remove_xattr(name))
    }
}
