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
//! `filter_private_names` methods, the shared classification-aware xattr
//! copy [`OverlayXattrPolicy::copy_eligible_xattrs`] (the single
//! classification-aware copy loop of the overlayfs tree, shared by the
//! copy-up and clear-empty paths through the [`XattrCopyPolicy`] failure
//! policy), and the four
//! `Inode`-trait xattr entries (`get_xattr`/`set_xattr`/`list_xattr`/
//! `remove_xattr`).
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
//! `is_overlay_private_xattr_name` (removed by pre-wave5 repair item 8)
//! excluded — so copy behavior is preserved while the classification
//! authority moves here.
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

/// The xattr-copy failure policy of the shared xattr copy
/// ([`OverlayXattrPolicy::copy_eligible_xattrs`]) — a small closed enum
/// (never a bool) selecting whether a source read or temp write that fails
/// (a denied access, a resource/I-O error, ...) aborts the copy (strict) or
/// degrades to warn-and-skip (best-effort).
///
/// The variants are named for the behavior they select; the two copy paths
/// map onto them as follows:
/// - [`XattrCopyPolicy::BestEffort`] (`P1-27` clear-empty path): the source
///   is the displaced upper directory of a clear-empty exchange, which is
///   being deleted, so its xattrs are moot — every copy error degrades and
///   the non-owner rmdir succeeds.
/// - [`XattrCopyPolicy::Strict`] (`P1-06` copy-up path): the copied object
///   is persisted, so a denied source read must fail the copy-up rather than
///   silently drop `security.*`/`trusted.*` metadata.
///
/// The list/read race (`ENODATA`/`ERANGE` — a concurrent xattr mutation
/// between the probe and the materialized read) always degrades to a skip;
/// it is a transient mutation, not a failure, under both policies.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::fs::fs_impls::overlayfs) enum XattrCopyPolicy {
    /// Best-effort source reads and temp writes: EVERY xattr-copy error — a
    /// denied source read or temp write (`EACCES`/`EPERM`), the transient
    /// list/read race (`ENODATA`/`ERANGE`), and resource/I-O failures
    /// (`ENOSPC`/`EIO`, ...) alike — degrades to warn-and-skip and the
    /// operation continues. Used by the clear-empty recipe (`ClearEmpty`
    /// path), whose source directory is about to be deleted: a pure rmdir
    /// must never abort on the xattr fidelity copy.
    BestEffort,
    /// Strict source reads and temp writes: a denied source read or a
    /// temp-write error (`EACCES`/`EPERM`, `ENOSPC`, `EIO`, ...) propagates
    /// and fails the copy — the wave-4 baseline for copy-up — so no
    /// `security.*`/`trusted.*` xattr is ever silently dropped. Only the
    /// transient list/read race (`ENODATA`/`ERANGE`) degrades to a skip.
    Strict,
}

/// Returns whether a SOURCE-read error of the shared xattr copy is skippable
/// under `policy` (the single source-read skip decision of
/// [`OverlayXattrPolicy::copy_eligible_xattrs`]; one helper for the
/// namespace-list and value-read arms, so the predicate can never diverge).
///
/// The list/read race (`ENODATA`/`ERANGE` — the source list/value changed
/// between the probe and the materialized read) is a transient mutation and
/// skips under BOTH policies. Under the best-effort
/// [`XattrCopyPolicy::BestEffort`] (the `ClearEmpty` path) EVERY source-read
/// error — a permission-denied read (`EACCES`/`EPERM`, the no-op
/// creator-credential seam, `mount/policy.rs`) as well as resource/I-O
/// failures (`ENOSPC`/`EIO`, ...) — degrades to warn-and-skip, because the
/// doomed source directory's xattr fidelity copy must never abort the pure
/// rmdir. Under the strict [`XattrCopyPolicy::Strict`] (`CopyUp` path) only
/// the transient race skips and every other error propagates (no silent
/// `security.*`/`trusted.*` loss on the persisted copy). Whitelist Rule B:
/// the exact predicate is executed by the two source-error arms of
/// `copy_eligible_xattrs` inside this meso. A free pure function rather than
/// an owner-local method: it reads no `self` state (pure `errno` × `policy`
/// decision), so an unused `&self` receiver would add noise without an
/// invariant to guard.
fn is_skippable_source_error(err: &Error, policy: XattrCopyPolicy) -> bool {
    policy == XattrCopyPolicy::BestEffort || matches!(err.error(), Errno::ENODATA | Errno::ERANGE)
}

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
    /// boundary filter that replaced the meso-04 local predicate
    /// `is_overlay_private_xattr_name` (removed when the classification
    /// authority moved here; no duplicated predicate survives).
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
            let is_private =
                core::str::from_utf8(name_bytes).is_ok_and(|name| self.is_private(name));
            if is_private {
                continue;
            }
            list_writer.write_fallible(&mut VmReader::from(name_bytes))?;
            list_writer.write_val(&0u8)?;
            bytes_written += name_bytes.len() + 1;
        }
        Ok(bytes_written)
    }

    /// Copies the eligible public xattrs of `source` onto `temp` (`P1-06`
    /// copy-up / `P1-27` clear-empty) — the single shared
    /// classification-aware xattr copy of the overlayfs tree.
    ///
    /// Enumerates the `User`, `Trusted`, and `Security` namespaces — the
    /// `System` namespace (`system.posix_acl_*`) is the `P2-05` ACL insertion
    /// point and stays excluded on every copy path — and filters
    /// overlay-private names through [`OverlayXattrPolicy::is_private`] (the
    /// meso-04 §7 supersession seam: the same name set the former copy-time
    /// predicate `is_overlay_private_xattr_name` excluded, so copy behavior
    /// is preserved while the classification authority lives here; the
    /// clear-empty recipe writes the temp's own `trusted.overlay.opaque`
    /// marker explicitly, so copying the displaced upper dir's marker would
    /// double-mark and is excluded by the same rule).
    ///
    /// The failure policy is selected by the caller through the closed
    /// [`XattrCopyPolicy`] enum:
    /// - **Best-effort source reads** ([`XattrCopyPolicy::BestEffort`],
    ///   `ClearEmpty` path): the source is the displaced upper directory of
    ///   a clear-empty exchange, which is being deleted, so its xattrs are
    ///   moot. EVERY source-read error — a denied read (`EACCES`/`EPERM` on
    ///   the namespace list or the value) as well as resource/I-O failures
    ///   (`ENOSPC`/`EIO`, ...) — degrades to "warn + skip" and the operation
    ///   continues, restoring the pre-C3 success path for a non-owner rmdir
    ///   of an owner-only xattr-carrying directory and keeping a pure rmdir
    ///   independent of the doomed directory's xattr fidelity copy.
    /// - **Strict source reads** ([`XattrCopyPolicy::Strict`], `CopyUp`
    ///   path): the copied object is persisted, so `EACCES`/`EPERM` on the
    ///   source namespace list or the value read PROPAGATES and the copy-up
    ///   fails — the wave-4 baseline — with NO silent
    ///   `security.*`/`trusted.*` loss.
    /// - **Race degradation (both policies):** a concurrent xattr mutation
    ///   between the probe and the materialized read surfaces as
    ///   `ENODATA`/`ERANGE` and degrades to "skip this xattr" (value read) or
    ///   "skip this namespace" (list probe), each with a `warn!`, never an
    ///   abort of the operation.
    /// - **Best-effort temp writes ([`XattrCopyPolicy::BestEffort`],
    ///   `ClearEmpty` path):** the temp `set_xattr` is part of the same
    ///   best-effort exchange — the displaced upper dir is being deleted and
    ///   the opaque temp is whiteouted — so ANY temp-write failure (a denied
    ///   write `EACCES`/`EPERM`, e.g. the temp's mode lacks owner-write for a
    ///   `user.*` xattr; the transient `ENODATA`/`ERANGE`; or a resource/I-O
    ///   error `ENOSPC`/`EIO`) degrades to "warn + skip this xattr" and the
    ///   clear-empty rmdir succeeds (the pre-C3 success path). Under the
    ///   strict [`XattrCopyPolicy::Strict`] a temp-write error still
    ///   propagates and fails the persisted copy.
    ///
    /// Recorded divergence (§3.0.5 item 4): the underlying source reads run
    /// under the CALLER's credentials because `with_creator_credentials_fn`
    /// is a documented no-op seam (`mount/policy.rs`); Linux copies under
    /// the creator's credentials and preserves these xattrs. The strict
    /// copy-up policy refuses the silent loss by propagating the denial —
    /// Linux's successful non-owner copy requires the §3.0.5 item-4
    /// credential-swap VFS seam; its landing is the full closure. Genuine
    /// xattr errors (an invalid name in the copied list) still hard-fail and
    /// abort the exchange before the rename.
    pub(in crate::fs::fs_impls::overlayfs) fn copy_eligible_xattrs(
        &self,
        source: &Arc<dyn Inode>,
        temp: &Arc<dyn Inode>,
        policy: XattrCopyPolicy,
    ) -> Result<()> {
        for namespace in [
            XattrNamespace::User,
            XattrNamespace::Trusted,
            XattrNamespace::Security,
        ] {
            // A source LIST error follows the selected policy: best-effort
            // degrades EVERY error to "skip this namespace's copy" with a
            // `warn!` (the clear-empty source is being deleted, so a pure
            // rmdir must never abort on the fidelity copy); strict
            // propagates every error except the list/read race (a persisted
            // copy must not lose the namespace). The list/read race
            // (`ENODATA`/`ERANGE` — the list grew between the probe and the
            // materialized read) degrades to "skip this namespace" under
            // BOTH policies (a transient mutation, never an abort).
            let names = match Self::list_xattr_names(source, namespace) {
                Ok(names) => names,
                Err(err) if is_skippable_source_error(&err, policy) => {
                    warn!(
                        "overlay xattr copy: source xattr list unavailable for {:?}; \
                         skipping this namespace: {:?}",
                        namespace, err
                    );
                    continue;
                }
                Err(err) => return Err(err),
            };
            for full_name in names
                .split(|&byte| byte == 0)
                .filter(|name| !name.is_empty())
            {
                let Ok(full_name) = core::str::from_utf8(full_name) else {
                    // The VFS `XattrName` is UTF-8 text; a non-UTF-8 list
                    // entry cannot be represented and is skipped.
                    continue;
                };
                if self.is_private(full_name) {
                    continue;
                }
                // The name is validated exactly once per copied xattr and the
                // validated `XattrName` is threaded through the value read and
                // the temp write — no re-validation of `full_name` and no
                // duplicated EINVAL error literal.
                let name = XattrName::try_from_full_name(full_name).ok_or_else(|| {
                    Error::with_message(Errno::EINVAL, "invalid xattr name in the copied list")
                })?;
                // Source value-read failures: the documented list/read race
                // (`ENODATA`/`ERANGE` — value removed or resized between the
                // probe and the materialized read) degrades to "skip this
                // xattr" under BOTH policies; under best-effort
                // ([`XattrCopyPolicy::BestEffort`], `ClearEmpty` path) EVERY
                // source value-read error — a denied read (`EACCES`/`EPERM`,
                // the no-op creator-credential seam, `mount/policy.rs`) as
                // well as resource/I-O failures (`ENOSPC`/`EIO`, ...) —
                // skips with a `warn!` (the clear-empty source being
                // deleted); strict propagates every error but the race (no
                // silent security-metadata loss on the persisted copy-up).
                let value = match Self::read_xattr_value(source, &name) {
                    Ok(value) => value,
                    Err(err) if is_skippable_source_error(&err, policy) => {
                        warn!("overlay xattr copy: skipping {}: {:?}", full_name, err);
                        continue;
                    }
                    Err(err) => return Err(err),
                };
                let mut reader = VmReader::from(value.as_slice()).to_fallible();
                // Best-effort temp writes (pre-wave5 round-7 repair; the
                // round-6 arm already covered `EACCES`/`EPERM`/`ENODATA`/
                // `ERANGE`): the displaced upper dir is being deleted and
                // the opaque temp is whiteouted, so ANY failed temp
                // `set_xattr` — a denied write (`EACCES`/`EPERM`, e.g. the
                // temp's mode lacks owner-write for a `user.*` xattr), the
                // transient race (`ENODATA`/`ERANGE`), or a resource/I-O
                // failure (`ENOSPC`/`EIO`, ...) — degrades to warn + skip
                // THIS xattr instead of aborting the whole clear-empty
                // exchange (a pure rmdir must never abort on the fidelity
                // copy). Strict keeps the wave-4 baseline: the persisted
                // object must not lose metadata, so every temp-write error
                // still hard-fails.
                match temp.set_xattr(name, &mut reader, XattrSetFlags::CREATE_OR_REPLACE) {
                    Err(err) if policy == XattrCopyPolicy::BestEffort => {
                        warn!(
                            "overlay xattr copy: skipping {} on temp: {:?}",
                            full_name, err
                        );
                        continue;
                    }
                    result => result?,
                }
            }
        }
        Ok(())
    }

    /// Lists the xattr names of one namespace on `source`.
    ///
    /// The VFS list convention is probed with a zero-capacity writer (returns
    /// the required size) and then materialized into an exactly sized buffer
    /// (ramfs/ext2 precedent; a size change between the two calls surfaces as
    /// `ERANGE`, which the caller treats as the documented list/read race and
    /// skips that namespace — never an abort). Whitelist Rule B: invoked once
    /// per namespace (three times per copy call) from
    /// [`OverlayXattrPolicy::copy_eligible_xattrs`].
    fn list_xattr_names(source: &Arc<dyn Inode>, namespace: XattrNamespace) -> Result<Vec<u8>> {
        let mut probe = VmWriter::from(&mut [] as &mut [u8]).to_fallible();
        let list_len = source.list_xattr(namespace, &mut probe)?;
        let mut names = vec![0u8; list_len];
        let mut list_writer = VmWriter::from(names.as_mut_slice()).to_fallible();
        let written = source.list_xattr(namespace, &mut list_writer)?;
        names.truncate(written);
        Ok(names)
    }

    /// Reads one xattr value from `source`.
    ///
    /// The value length is probed with a zero-capacity writer and the value is
    /// then materialized into an exactly sized buffer. `XattrName` is not
    /// `Copy` and carries no `Clone` (frozen VFS surface), so each `get_xattr`
    /// call takes its own owned view; both views are re-borrowed from the
    /// caller's already-validated name (validated exactly once in the copy
    /// loop), so the helper carries no validation and no error site of its own
    /// — the single `ok_or_else` lives in the copy loop. Whitelist Rule B:
    /// invoked once per listed name (multiple times per copy call) from
    /// [`OverlayXattrPolicy::copy_eligible_xattrs`].
    fn read_xattr_value(source: &Arc<dyn Inode>, name: &XattrName<'_>) -> Result<Vec<u8>> {
        // `XattrName` is not `Copy`/`Clone`, so each `get_xattr` re-borrows a
        // thin owned view of the same full name. The copy loop validated
        // `name` exactly once; re-parsing the same full name cannot fail (the
        // recorded hard-invariant `unreachable!` precedent of the tree, never
        // `.unwrap()`/`.expect()`).
        let reborrow_fn = || match XattrName::try_from_full_name(name.full_name()) {
            Some(name) => name,
            None => unreachable!("the copy loop validated this xattr name"),
        };
        let mut probe = VmWriter::from(&mut [] as &mut [u8]).to_fallible();
        let value_len = source.get_xattr(reborrow_fn(), &mut probe)?;
        let mut value = vec![0u8; value_len];
        let mut value_writer = VmWriter::from(value.as_mut_slice()).to_fallible();
        let written = source.get_xattr(reborrow_fn(), &mut value_writer)?;
        value.truncate(written);
        Ok(value)
    }
}

impl OverlayInode {
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
    pub(in crate::fs::fs_impls::overlayfs) fn get_xattr_impl(
        &self,
        name: XattrName,
        value_writer: &mut VmWriter,
    ) -> Result<usize> {
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
    pub(in crate::fs::fs_impls::overlayfs) fn set_xattr_impl(
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
    pub(in crate::fs::fs_impls::overlayfs) fn list_xattr_impl(
        &self,
        namespace: XattrNamespace,
        list_writer: &mut VmWriter,
    ) -> Result<usize> {
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
    pub(in crate::fs::fs_impls::overlayfs) fn remove_xattr_impl(
        &self,
        name: XattrName,
    ) -> Result<()> {
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
