// SPDX-License-Identifier: MPL-2.0
#![short_vis_path::add(overlayfs)]

//! The xattr two-path policy: the private record path and the passthrough
//! path.
//!
//! Every xattr full name is classified into one of two classes by prefix
//! only:
//!
//! - `Private`: the name starts with the mount's selected private prefix —
//!   `trusted.overlay.` by default, `user.overlay.` in `userxattr` mode.
//!   These are the overlay's own records (origin, opaque, whiteout, impure,
//!   uuid); they never cross copy-up and are hidden from the visible list.
//! - `Passthrough`: every other name (`user.plain.any`, `security.selinux`,
//!   `trusted.backup.notes`, ...); passed through unchanged, never
//!   interpreted.
//!
//! # The two paths
//!
//! - **Private path** ([`OverlayInode::set_overlay_xattr`] plus the raw
//!   record reads): the name reaches the real object unchanged — the escape
//!   infix is never inserted. Every internal overlay write routes through
//!   this entry, so the un-escaped-name invariant has one enforcement point.
//! - **Passthrough path** (the `*_impl` entries): an own-prefix name is
//!   dislocated one segment down ([`ESCAPE_INFIX`] inserted right after the
//!   selected prefix) before it reaches the real authority — unconditionally,
//!   even for a name that already carries the infix. The list transform
//!   ([`present_xattr_names`]) is the inverse map: own private records are
//!   hidden and one infix segment is stripped per layer.
//!
//! Stacked same-prefix overlays therefore physically layer their records by
//! infix-segment count: the count equals the number of overlays between the
//! record's owner and the backing filesystem, and each layer's passthrough
//! path adds exactly one segment while each layer's list transform strips
//! exactly one.
//!
//! # userxattr feature exclusions (enforced at mount time)
//!
//! `userxattr` excludes the `redirect_dir`/`metacopy` features. The option
//! verify phase rejects the explicit combinations — `userxattr` +
//! `redirect_dir`≠`nofollow` → `EINVAL`; `userxattr` + `metacopy=on` →
//! `EINVAL` — and both features remain unimplemented, so every other
//! explicit request degrades with a disclosed one-shot warning. The
//! exclusivity contract is recorded here because the private-prefix
//! decision and the option surface must stay consistent.
//!
//! ## References
//!
//! - <https://elixir.bootlin.com/linux/v6.17/source/fs/overlayfs/xattrs.c#L157-L180>
//!   (Linux `ovl_xattr_escape_name` infix insertion and `EOPNOTSUPP` limit)
//! - <https://elixir.bootlin.com/linux/v6.17/source/fs/overlayfs/xattrs.c#L182-L218>
//!   (Linux `ovl_own_xattr_{get,set}` unconditional escape)
//! - <https://elixir.bootlin.com/linux/v6.17/source/fs/overlayfs/params.c#L956-L976>
//!   (Linux userxattr redirect/metacopy exclusivity and default disabling)

use super::{OverlayInode, ReaddirIndex, permission::AccessType};
use crate::{
    fs::{
        file::Permission,
        fs_impls::overlayfs::fs::OverlayFs,
        vfs::{
            inode::Inode,
            xattr::{
                XATTR_LIST_MAX_LEN, XATTR_NAME_MAX_LEN, XattrName, XattrNamespace, XattrSetFlags,
            },
        },
    },
    prelude::*,
};

/// The default private prefix; writing it needs `CAP_SYS_ADMIN`, which
/// stops non-root users from modifying the overlay's own records.
const TRUSTED_OVERLAY_PREFIX: &str = "trusted.overlay.";

/// The `userxattr`-mode private prefix; the unprivileged workaround. It
/// cannot prevent users from modifying the records under it.
const USER_OVERLAY_PREFIX: &str = "user.overlay.";

/// The one-segment escape infix inserted right after the selected private
/// prefix by the passthrough path (proposal §13:
/// `insert_str(selected_prefix.len(), "overlay.")`).
const ESCAPE_INFIX: &str = "overlay.";

// Record full names, trusted-mode and userxattr-mode pairs (U1 closure);
// the on-disk record surface of the five overlay record kinds.
const TRUSTED_OVERLAY_ORIGIN: &str = "trusted.overlay.origin";
const USER_OVERLAY_ORIGIN: &str = "user.overlay.origin";
const TRUSTED_OVERLAY_OPAQUE: &str = "trusted.overlay.opaque";
const USER_OVERLAY_OPAQUE: &str = "user.overlay.opaque";
const TRUSTED_OVERLAY_WHITEOUT: &str = "trusted.overlay.whiteout";
const USER_OVERLAY_WHITEOUT: &str = "user.overlay.whiteout";
const TRUSTED_OVERLAY_IMPURE: &str = "trusted.overlay.impure";
const USER_OVERLAY_IMPURE: &str = "user.overlay.impure";
const TRUSTED_OVERLAY_UUID: &str = "trusted.overlay.uuid";
const USER_OVERLAY_UUID: &str = "user.overlay.uuid";

/// The opaque marker value; the reader requires the first byte `b'y'`.
const OPAQUE_MARKER_VALUE: &[u8] = b"y";

/// The whiteout marker value; the reader requires the first byte `b'y'`.
pub(super) const WHITEOUT_MARKER_VALUE: &[u8] = b"y";

/// The impure marker value; the reader is presence-based.
const IMPURE_MARKER_VALUE: &[u8] = b"y";

/// The overlay private record kinds (closed set; extended with the table).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in overlayfs) enum OverlayRecordName {
    /// Durable lower-source origin record (`.../origin`).
    Origin,
    /// Directory merge barrier (`.../opaque`).
    Opaque,
    /// Whiteout marker (`.../whiteout`).
    Whiteout,
    /// Readdir-cache hint (`.../impure`).
    Impure,
    /// Persisted overlay UUID record (`.../uuid`).
    Uuid,
}

/// Returns the parsed [`XattrName`] of `record`'s full name under `prefix`.
///
/// Pure input→output mapping: no state, no side effects; every
/// `(record, prefix)` pair is a module-owned const name that is
/// namespace-parsable by construction, so the `EINVAL` arm is defensively
/// unreachable.
pub(in overlayfs) fn overlay_record_name(
    record: OverlayRecordName,
    prefix: OverlayXattrPrefix,
) -> Result<XattrName<'static>> {
    let full_name: &'static str = match (record, prefix) {
        (OverlayRecordName::Origin, OverlayXattrPrefix::Trusted) => TRUSTED_OVERLAY_ORIGIN,
        (OverlayRecordName::Origin, OverlayXattrPrefix::User) => USER_OVERLAY_ORIGIN,
        (OverlayRecordName::Opaque, OverlayXattrPrefix::Trusted) => TRUSTED_OVERLAY_OPAQUE,
        (OverlayRecordName::Opaque, OverlayXattrPrefix::User) => USER_OVERLAY_OPAQUE,
        (OverlayRecordName::Whiteout, OverlayXattrPrefix::Trusted) => TRUSTED_OVERLAY_WHITEOUT,
        (OverlayRecordName::Whiteout, OverlayXattrPrefix::User) => USER_OVERLAY_WHITEOUT,
        (OverlayRecordName::Impure, OverlayXattrPrefix::Trusted) => TRUSTED_OVERLAY_IMPURE,
        (OverlayRecordName::Impure, OverlayXattrPrefix::User) => USER_OVERLAY_IMPURE,
        (OverlayRecordName::Uuid, OverlayXattrPrefix::Trusted) => TRUSTED_OVERLAY_UUID,
        (OverlayRecordName::Uuid, OverlayXattrPrefix::User) => USER_OVERLAY_UUID,
    };
    XattrName::try_from_full_name(full_name)
        .ok_or_else(|| Error::with_message(Errno::EINVAL, "invalid overlay record xattr name"))
}

/// The two-way classification of an xattr full name (proposal §13; do NOT
/// rename — deliberately avoids clashing with the VFS `XattrName` struct).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum XattrClass {
    /// Starts with the mount's selected private prefix.
    Private,
    /// Every other name (`user.plain.any`, `security.selinux`,
    /// `trusted.backup.notes`, ...): passed through, never interpreted.
    Passthrough,
}

/// The selected private-prefix namespace of a mount (the `userxattr` mount
/// option's decision state, stored in `MountPolicy`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in overlayfs) enum OverlayXattrPrefix {
    /// `trusted.overlay.` (default; writing it needs `CAP_SYS_ADMIN`).
    Trusted,
    /// `user.overlay.` (`userxattr` mode; the unprivileged workaround).
    User,
}

impl OverlayXattrPrefix {
    /// Returns the selected private prefix string.
    pub(in overlayfs) fn as_str(self) -> &'static str {
        match self {
            OverlayXattrPrefix::Trusted => TRUSTED_OVERLAY_PREFIX,
            OverlayXattrPrefix::User => USER_OVERLAY_PREFIX,
        }
    }
}

/// Classifies a full name into `Private`/`Passthrough` by prefix only.
fn classify(full_name: &str, prefix: OverlayXattrPrefix) -> XattrClass {
    if full_name.starts_with(prefix.as_str()) {
        XattrClass::Private
    } else {
        XattrClass::Passthrough
    }
}

/// Returns whether `full_name` carries the selected private prefix.
///
/// This is the copy-time boundary filter: a name classifying `Private`
/// under the copying mount's selected prefix never crosses copy-up
/// (invariant I5).
fn is_private(full_name: &str, prefix: OverlayXattrPrefix) -> bool {
    matches!(classify(full_name, prefix), XattrClass::Private)
}

/// Returns the full name actually used against the real authority for the
/// passthrough entries: an own-prefix name with one [`ESCAPE_INFIX`] segment
/// inserted at `selected_prefix.len()`, or the unchanged name.
///
/// Errors `EOPNOTSUPP` when the escaped form would exceed
/// [`XATTR_NAME_MAX_LEN`] (Linux `ovl_xattr_escape_name` parity). The
/// insertion happens unconditionally on an own-prefix match — even for a
/// name that already carries the infix — so each overlay layer adds exactly
/// one segment (invariant I3, Linux `ovl_own_xattr_{get,set}` parity).
fn used_full_name(name: &XattrName, selected_prefix: &str) -> Result<String> {
    let full_name = name.full_name();
    if !full_name.starts_with(selected_prefix) {
        return Ok(String::from(full_name));
    }
    if full_name.len() + ESCAPE_INFIX.len() > XATTR_NAME_MAX_LEN {
        return_errno_with_message!(
            Errno::EOPNOTSUPP,
            "the escaped overlay xattr name exceeds the xattr name length limit"
        );
    }
    let mut used_name = String::from(full_name);
    used_name.insert_str(selected_prefix.len(), ESCAPE_INFIX);
    Ok(used_name)
}

/// Presents a raw real-authority name list for the viewer above this
/// overlay: drops own-prefix names without the escape infix (private
/// records), strips one [`ESCAPE_INFIX`] segment from own-prefix names that
/// carry it right after the prefix, and passes every other name through.
///
/// With a zero-capacity `list_writer` (the size probe) it reports the
/// transformed total size without writing; `ERANGE` on mid-stream overflow.
/// This is the inverse map of the passthrough escape (invariant I1): each
/// overlay layer's list transform strips exactly one segment.
fn present_xattr_names(
    raw_list: &[u8],
    prefix: OverlayXattrPrefix,
    list_writer: &mut VmWriter,
) -> Result<usize> {
    let selected_prefix = prefix.as_str();
    let mut bytes_written = 0;
    // The strip branch materializes the presented name: the selected private
    // prefix is KEPT and one `ESCAPE_INFIX` segment is removed — the inverse
    // map of the passthrough escape (Linux `ovl_listxattr` memmove parity:
    // the prefix is retained, only the infix is moved out). The buffer is
    // reused across entries; only the strip branch writes it.
    let mut stripped_name = String::new();
    for name_bytes in raw_list.split(|&byte| byte == 0) {
        if name_bytes.is_empty() {
            continue;
        }
        // A non-UTF-8 list entry cannot carry the UTF-8 selected prefix and
        // is passed through unchanged.
        let presented: &[u8] = match core::str::from_utf8(name_bytes) {
            Ok(name) if name.starts_with(selected_prefix) => {
                match name[selected_prefix.len()..].strip_prefix(ESCAPE_INFIX) {
                    // Own escaped record: present `selected_prefix` followed
                    // by the name with one infix segment removed.
                    Some(stripped_suffix) => {
                        stripped_name.clear();
                        stripped_name.push_str(selected_prefix);
                        stripped_name.push_str(stripped_suffix);
                        stripped_name.as_bytes()
                    }
                    // Own private record: hidden from the viewer above.
                    None => continue,
                }
            }
            _ => name_bytes,
        };
        let entry_len = presented.len() + 1;
        if list_writer.avail() == 0 {
            bytes_written += entry_len;
            continue;
        }
        if entry_len > list_writer.avail() {
            return_errno_with_message!(
                Errno::ERANGE,
                "the xattr list buffer is too small for the presented list"
            );
        }
        list_writer.write_fallible(&mut VmReader::from(presented))?;
        list_writer.write_val(&0u8)?;
        bytes_written += entry_len;
    }
    Ok(bytes_written)
}

/// The read semantics of an overlay marker xattr.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MarkerReadSemantics {
    /// Presence probe (impure): any successful read counts as present, and
    /// `ERANGE` still counts as present.
    Presence,
    /// Value probe (whiteout/opaque): exactly one byte `b'y'` counts.
    ValueY,
}

/// Reads one overlay marker xattr under `semantics`.
///
/// `ENODATA`/`EOPNOTSUPP` mean "marker absent"; `ERANGE` follows the
/// semantics (present for `Presence`, absent for `ValueY`); other errors
/// propagate unchanged.
pub(super) fn has_marker(
    real_inode: &Arc<dyn Inode>,
    name: XattrName<'static>,
    semantics: MarkerReadSemantics,
) -> Result<bool> {
    let mut value = [0u8; 1];
    let mut writer = VmWriter::from(value.as_mut_slice()).to_fallible();
    match real_inode.get_xattr(name, &mut writer) {
        Ok(written) => match semantics {
            MarkerReadSemantics::Presence => Ok(true),
            MarkerReadSemantics::ValueY => Ok(written == 1 && value[0] == b'y'),
        },
        Err(err) if err.error() == Errno::ERANGE => {
            Ok(matches!(semantics, MarkerReadSemantics::Presence))
        }
        Err(err) if matches!(err.error(), Errno::ENODATA | Errno::EOPNOTSUPP) => Ok(false),
        Err(err) => Err(err),
    }
}

/// Returns whether `real_dir` carries the persisted impure marker.
///
/// Presence probe on the real upper directory: the marker is interpreted
/// by presence, not by value.
fn has_impure_marker(real_dir: &Arc<dyn Inode>, prefix: OverlayXattrPrefix) -> Result<bool> {
    has_marker(
        real_dir,
        overlay_record_name(OverlayRecordName::Impure, prefix)?,
        MarkerReadSemantics::Presence,
    )
}

impl OverlayInode {
    /// Private path (proposal §13): writes `name` to the real object `real`
    /// unchanged — the name is never escaped here. All internal overlay
    /// writes (markers, origin, uuid) route through this entry so the
    /// un-escaped-name invariant has one enforcement point; a nested lower
    /// overlay's passthrough path adds exactly one segment per layer.
    pub(in overlayfs) fn set_overlay_xattr(
        real: &Arc<dyn Inode>,
        name: XattrName,
        value_reader: &mut VmReader,
        flags: XattrSetFlags,
    ) -> Result<()> {
        real.set_xattr(name, value_reader, flags)
    }

    /// Persists the impure marker on the real upper directory `real_dir`.
    ///
    /// Read-first idempotent: an already-marked directory is a no-op. The
    /// write routes through the private path
    /// ([`OverlayInode::set_overlay_xattr`]) with the record's un-escaped
    /// name.
    pub(super) fn set_impure_marker(
        real_dir: &Arc<dyn Inode>,
        prefix: OverlayXattrPrefix,
    ) -> Result<()> {
        if has_impure_marker(real_dir, prefix)? {
            return Ok(());
        }
        let name = overlay_record_name(OverlayRecordName::Impure, prefix)?;
        let mut marker_reader = VmReader::from(IMPURE_MARKER_VALUE).to_fallible();
        Self::set_overlay_xattr(
            real_dir,
            name,
            &mut marker_reader,
            XattrSetFlags::CREATE_OR_REPLACE,
        )
    }

    /// Removes the impure marker from the real upper directory `real_dir`.
    /// Absence is already the cleared state, so clearing is idempotent.
    fn clear_impure_marker(real_dir: &Arc<dyn Inode>, prefix: OverlayXattrPrefix) -> Result<()> {
        let name = overlay_record_name(OverlayRecordName::Impure, prefix)?;
        match real_dir.remove_xattr(name) {
            Ok(()) => Ok(()),
            Err(err) if err.error() == Errno::ENODATA => Ok(()),
            Err(err) => Err(err),
        }
    }

    /// Refreshes the persisted impure marker after mutation,
    /// clearing it when no visible child keeps a non-empty lower stack.
    ///
    /// The clear is valid only under the immutable-lower premise:
    /// the lower stack is one the overlay never writes.
    /// Mounts with `default_permissions` do not implement this premise yet.
    ///
    /// A residual check-use race with an external lower writer
    /// cannot be closed by an overlay lock, so it is deliberately not handled.
    fn refresh_impure_marker(&self, index: &mut Option<ReaddirIndex>) -> Result<()> {
        // Upper-present gate: the marker lives only on real upper
        // directories; a lower-only directory cannot carry one.
        let Some(upper_real) = self.upper.get() else {
            return Ok(());
        };
        let prefix = self.fs_arc()?.policy().xattr_prefix();
        if !has_impure_marker(upper_real.real_inode(), prefix)? {
            return Ok(());
        }
        let index = index.as_mut().ok_or_else(|| {
            Error::with_message(Errno::ENOTDIR, "the overlay inode is not a directory")
        })?;
        let facts = self.real_object_stack();
        self.ensure_readdir_index(&facts, index)?;
        let children = index.visible_inodes();
        for child in &children {
            if !child.lowers.is_empty() {
                return Ok(());
            }
        }
        Self::clear_impure_marker(upper_real.real_inode(), prefix)
    }

    /// Best-effort variant of [`OverlayInode::refresh_impure_marker`] for
    /// mutation tails: the mutation has already committed, so a refresh
    /// failure is logged and ignored.
    pub(super) fn refresh_impure_marker_best_effort(
        &self,
        index: &mut Option<ReaddirIndex>,
        operation: &'static str,
    ) {
        if let Err(err) = self.refresh_impure_marker(index) {
            warn!(
                "overlay {}: the impure-marker refresh failed (best-effort): {:?}",
                operation, err
            );
        }
    }
}

impl OverlayFs {
    /// Writes the opaque marker onto `target` after the private-xattr
    /// capability gate; `unsupported_message` is the caller-scoped static
    /// `EOPNOTSUPP` message (the two call sites keep their distinct text).
    ///
    /// The write routes through the private path
    /// ([`OverlayInode::set_overlay_xattr`]) with the record's un-escaped
    /// name.
    pub(super) fn set_opaque_marker(
        &self,
        target: &Arc<dyn Inode>,
        unsupported_message: &'static str,
    ) -> Result<()> {
        let can_store_private_xattr = self
            .policy()
            .upper_capabilities()
            .is_some_and(|caps| caps.can_store_private_xattr());
        if !can_store_private_xattr {
            return Err(Error::with_message(Errno::EOPNOTSUPP, unsupported_message));
        }
        let marker_name =
            overlay_record_name(OverlayRecordName::Opaque, self.policy().xattr_prefix())?;
        let mut marker_reader = VmReader::from(OPAQUE_MARKER_VALUE).to_fallible();
        OverlayInode::set_overlay_xattr(
            target,
            marker_name,
            &mut marker_reader,
            XattrSetFlags::CREATE_OR_REPLACE,
        )
    }
}

impl OverlayInode {
    /// Passthrough `get`: an own-prefix name is dislocated one segment down
    /// ([`used_full_name`]) before the real read, so an absent dislocated
    /// name surfaces as the real authority's `ENODATA` — read-side hiding
    /// without a refusal class (Linux `ovl_own_xattr_get` parity).
    pub(super) fn get_xattr_impl(
        &self,
        name: XattrName,
        value_writer: &mut VmWriter,
    ) -> Result<usize> {
        let prefix = self.fs_arc()?.policy().xattr_prefix();
        let used_name = used_full_name(&name, prefix.as_str())?;
        // The escaped name is namespace-parsable by construction (the
        // insertion happens inside the trusted/user namespace), so the
        // `EINVAL` arm is defensively unreachable.
        let used = XattrName::try_from_full_name(&used_name).ok_or_else(|| {
            Error::with_message(Errno::EINVAL, "invalid escaped overlay xattr name")
        })?;
        self.check_permission(AccessType::ReadOnly, Permission::MAY_READ)?;
        self.delegate_to_real(|real| real.get_xattr(used, value_writer))
    }

    /// Passthrough `set`: an own-prefix name is dislocated one segment down
    /// ([`used_full_name`]) before the real write, so a nested lower
    /// overlay's records stay per-layer invisible (Linux
    /// `ovl_own_xattr_set` parity). No refusal class for own-prefix names.
    pub(super) fn set_xattr_impl(
        &self,
        name: XattrName,
        value_reader: &mut VmReader,
        flags: XattrSetFlags,
    ) -> Result<()> {
        let prefix = self.fs_arc()?.policy().xattr_prefix();
        let used_name = used_full_name(&name, prefix.as_str())?;
        // See `get_xattr_impl`: the `EINVAL` arm is defensively unreachable.
        let used = XattrName::try_from_full_name(&used_name).ok_or_else(|| {
            Error::with_message(Errno::EINVAL, "invalid escaped overlay xattr name")
        })?;
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        self.delegate_to_real(|real| real.set_xattr(used, value_reader, flags))
    }

    /// Passthrough `list`: the raw real-authority list is presented through
    /// [`present_xattr_names`] — own private records hidden, own escaped
    /// records stripped one segment, others unchanged. No local DAC demand
    /// applies (Linux `vfs_listxattr` parity): visibility is per-name
    /// filtering.
    pub(super) fn list_xattr_impl(
        &self,
        namespace: XattrNamespace,
        list_writer: &mut VmWriter,
    ) -> Result<usize> {
        let prefix = self.fs_arc()?.policy().xattr_prefix();
        self.delegate_to_real(|real| {
            let mut raw_list = vec![0u8; XATTR_LIST_MAX_LEN];
            let mut raw_writer = VmWriter::from(&mut raw_list[..]).to_fallible();
            let list_len = real.list_xattr(namespace, &mut raw_writer)?;
            present_xattr_names(&raw_list[..list_len], prefix, list_writer)
        })
    }

    /// Passthrough `remove`: an own-prefix name is dislocated one segment
    /// down ([`used_full_name`]) before the real removal, mirroring the
    /// get/set dislocation; `ENODATA` propagates when the dislocated name
    /// is absent.
    pub(super) fn remove_xattr_impl(&self, name: XattrName) -> Result<()> {
        let prefix = self.fs_arc()?.policy().xattr_prefix();
        let used_name = used_full_name(&name, prefix.as_str())?;
        // See `get_xattr_impl`: the `EINVAL` arm is defensively unreachable.
        let used = XattrName::try_from_full_name(&used_name).ok_or_else(|| {
            Error::with_message(Errno::EINVAL, "invalid escaped overlay xattr name")
        })?;
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        self.delegate_to_real(|real| real.remove_xattr(used))
    }
}

/// The xattr-copy failure policy of the shared xattr copy
/// ([`OverlayInode::copy_eligible_xattrs`]): strict aborts on a denied
/// source read or temp write, best-effort warns and skips; the transient
/// list/read race (`ENODATA`/`ERANGE`) always degrades to a skip.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum XattrCopyPolicy {
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
    /// and fails the copy — the copy-up baseline — so no
    /// `security.*`/`trusted.*` xattr is ever silently dropped. Only the
    /// transient list/read race (`ENODATA`/`ERANGE`) degrades to a skip.
    Strict,
}

fn is_skippable_source_error(err: &Error, copy_policy: XattrCopyPolicy) -> bool {
    copy_policy == XattrCopyPolicy::BestEffort
        || matches!(err.error(), Errno::ENODATA | Errno::ERANGE)
}

impl OverlayInode {
    /// Copies the eligible public xattrs of `source` onto `temp`
    /// (copy-up / clear-empty) in the `User`/`Trusted`/`Security` namespaces,
    /// dropping the copying mount's own-prefix names (invariant I5).
    ///
    /// `prefix` is the copying mount's selected prefix. No
    /// creator-credential scope is currently available,
    /// so source reads run under the caller's credentials;
    /// a denied read therefore propagates as an error
    /// instead of silently dropping `security.*`/`trusted.*` xattrs.
    pub(super) fn copy_eligible_xattrs(
        source: &Arc<dyn Inode>,
        temp: &Arc<dyn Inode>,
        copy_policy: XattrCopyPolicy,
        prefix: OverlayXattrPrefix,
    ) -> Result<()> {
        for namespace in [
            XattrNamespace::User,
            XattrNamespace::Trusted,
            XattrNamespace::Security,
        ] {
            let names = match list_xattr_names(source, namespace) {
                Ok(names) => names,
                Err(err) if is_skippable_source_error(&err, copy_policy) => {
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
                if is_private(full_name, prefix) {
                    continue;
                }
                // The parsed `XattrName` is reused for the value read and the
                // temp write.
                let Some(name) = XattrName::try_from_full_name(full_name) else {
                    warn!(
                        "overlay xattr copy: skipping unparsable xattr name: {}",
                        full_name
                    );
                    continue;
                };
                if name.namespace() != namespace {
                    continue;
                }
                let value = match read_xattr_value(source, &name) {
                    Ok(value) => value,
                    Err(err) if is_skippable_source_error(&err, copy_policy) => {
                        warn!("overlay xattr copy: skipping {}: {:?}", full_name, err);
                        continue;
                    }
                    Err(err) => return Err(err),
                };
                let mut reader = VmReader::from(value.as_slice()).to_fallible();
                match temp.set_xattr(name, &mut reader, XattrSetFlags::CREATE_OR_REPLACE) {
                    Err(err) if copy_policy == XattrCopyPolicy::BestEffort => {
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
}

/// Lists the xattr names of one namespace on `source`, probing the
/// required size with a zero-capacity writer before materializing the
/// list; a size change between the two calls surfaces as `ERANGE`.
fn list_xattr_names(source: &Arc<dyn Inode>, namespace: XattrNamespace) -> Result<Vec<u8>> {
    let mut probe = VmWriter::from(&mut [] as &mut [u8]).to_fallible();
    let list_len = source.list_xattr(namespace, &mut probe)?;
    let mut names = vec![0u8; list_len];
    let mut list_writer = VmWriter::from(names.as_mut_slice()).to_fallible();
    let written = source.list_xattr(namespace, &mut list_writer)?;
    names.truncate(written);
    Ok(names)
}

/// Reads one xattr value from `source`, probing the required size with a
/// zero-capacity writer before materializing the value.
fn read_xattr_value(source: &Arc<dyn Inode>, name: &XattrName<'_>) -> Result<Vec<u8>> {
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

#[cfg(ktest)]
mod test {
    // SPDX-License-Identifier: MPL-2.0

    //! Unit tests for the pure xattr name mapping (U-2): classification, escape,
    //! and the list-present transform.
    //!
    //! Every expectation below is the frozen U-2 case tables of the test-assets
    //! design (`test-assets-20260831` §3.2). The tests assert the pure mapping
    //! only: no filesystem, VFS, block, or I/O fixture is constructed.

    use ostd::prelude::ktest;

    use super::*;

    /// Parses a namespace-parsable full name into an `XattrName`.
    fn xname(full_name: &'static str) -> XattrName<'static> {
        XattrName::try_from_full_name(full_name).unwrap()
    }

    /// Builds an own-prefix name of exactly `full_len` bytes (prefix + `a` fill).
    fn own_name_of_len(selected_prefix: &str, full_len: usize) -> String {
        let mut name = String::from(selected_prefix);
        while name.len() < full_len {
            name.push('a');
        }
        name
    }

    /// Presents `raw` with a `buffer_len`-byte buffer expecting success;
    /// returns (reported total length, written buffer prefix).
    fn present_expect_ok(
        raw: &[u8],
        prefix: OverlayXattrPrefix,
        buffer_len: usize,
    ) -> (usize, Vec<u8>) {
        let mut buffer = vec![0u8; buffer_len];
        let mut writer = VmWriter::from(buffer.as_mut_slice()).to_fallible();
        let total = present_xattr_names(raw, prefix, &mut writer).unwrap();
        let written = total.min(buffer_len);
        (total, buffer[..written].to_vec())
    }

    /// Presents `raw` with a `buffer_len`-byte buffer expecting `ERANGE`.
    fn present_expect_erange(raw: &[u8], prefix: OverlayXattrPrefix, buffer_len: usize) {
        let mut buffer = vec![0u8; buffer_len];
        let mut writer = VmWriter::from(buffer.as_mut_slice()).to_fallible();
        let err = present_xattr_names(raw, prefix, &mut writer).unwrap_err();
        assert_eq!(err.error(), Errno::ERANGE);
    }

    #[ktest]
    fn classify_private_by_selected_prefix() {
        assert_eq!(
            classify("trusted.overlay.fsz", OverlayXattrPrefix::Trusted),
            XattrClass::Private
        );
        assert_eq!(
            classify("user.overlay.fsz", OverlayXattrPrefix::User),
            XattrClass::Private
        );
        // No trailing dot: past the prefix.
        assert_eq!(
            classify("trusted.overlay", OverlayXattrPrefix::Trusted),
            XattrClass::Passthrough
        );
        // Prefix boundary.
        assert_eq!(
            classify("trusted.overlayfsrz", OverlayXattrPrefix::Trusted),
            XattrClass::Passthrough
        );
        // Exactly the prefix.
        assert_eq!(
            classify("trusted.overlay.", OverlayXattrPrefix::Trusted),
            XattrClass::Private
        );
        // Case-sensitive.
        assert_eq!(
            classify("Trusted.overlay.x", OverlayXattrPrefix::Trusted),
            XattrClass::Passthrough
        );
        // Cross-prefix.
        assert_eq!(
            classify("user.overlay.x", OverlayXattrPrefix::Trusted),
            XattrClass::Passthrough
        );
        assert_eq!(
            classify("trusted.overlay.x", OverlayXattrPrefix::User),
            XattrClass::Passthrough
        );
        assert_eq!(
            classify("user.plain.any", OverlayXattrPrefix::Trusted),
            XattrClass::Passthrough
        );
        assert_eq!(
            classify("security.selinux", OverlayXattrPrefix::Trusted),
            XattrClass::Passthrough
        );
        assert_eq!(
            classify("trusted.backup.notes", OverlayXattrPrefix::Trusted),
            XattrClass::Passthrough
        );
        // The copy-time boundary filter (I5).
        assert!(is_private("trusted.overlay.opaque", OverlayXattrPrefix::Trusted));
        assert!(!is_private("user.plain", OverlayXattrPrefix::Trusted));
    }

    #[ktest]
    fn used_full_name_passes_foreign_through() {
        // A foreign name is used unchanged.
        assert_eq!(
            used_full_name(&xname("user.plain.any"), OverlayXattrPrefix::Trusted.as_str()).unwrap(),
            "user.plain.any"
        );
        // A cross-prefix own-looking name is foreign here.
        assert_eq!(
            used_full_name(&xname("user.overlay.x"), OverlayXattrPrefix::Trusted.as_str()).unwrap(),
            "user.overlay.x"
        );
        // The length limit is enforced only on the escape path (frozen
        // asymmetry): a foreign name of length 300 passes through unchanged.
        let foreign = own_name_of_len("user.", 300);
        let used = used_full_name(
            &XattrName::try_from_full_name(foreign.as_str()).unwrap(),
            OverlayXattrPrefix::Trusted.as_str(),
        )
        .unwrap();
        assert_eq!(used, foreign);
    }

    #[ktest]
    fn used_full_name_escapes_own_prefix_unconditionally() {
        // One `overlay.` infix is inserted right after the prefix.
        assert_eq!(
            used_full_name(&xname("trusted.overlay.fsz"), OverlayXattrPrefix::Trusted.as_str())
                .unwrap(),
            "trusted.overlay.overlay.fsz"
        );
        // The insertion is unconditional, even for an already-escaped name.
        assert_eq!(
            used_full_name(
                &xname("trusted.overlay.overlay.fsz"),
                OverlayXattrPrefix::Trusted.as_str()
            )
            .unwrap(),
            "trusted.overlay.overlay.overlay.fsz"
        );
        // Exactly-the-prefix boundary.
        assert_eq!(
            used_full_name(&xname("trusted.overlay."), OverlayXattrPrefix::Trusted.as_str())
                .unwrap(),
            "trusted.overlay.overlay."
        );
        // The same escape under the `userxattr` prefix.
        assert_eq!(
            used_full_name(&xname("user.overlay.fsz"), OverlayXattrPrefix::User.as_str()).unwrap(),
            "user.overlay.overlay.fsz"
        );
    }

    #[ktest]
    fn used_full_name_enforces_name_length_limit_on_escape() {
        // An own name of full length 247 escapes to length 255, which fits
        // `XATTR_NAME_MAX_LEN`.
        let fits = own_name_of_len(OverlayXattrPrefix::Trusted.as_str(), 247);
        let used = used_full_name(
            &XattrName::try_from_full_name(fits.as_str()).unwrap(),
            OverlayXattrPrefix::Trusted.as_str(),
        )
        .unwrap();
        assert_eq!(used.len(), XATTR_NAME_MAX_LEN);
        // An own name of full length 248 escapes to length 256, exceeding the
        // limit.
        let exceeds = own_name_of_len(OverlayXattrPrefix::Trusted.as_str(), 248);
        let err = used_full_name(
            &XattrName::try_from_full_name(exceeds.as_str()).unwrap(),
            OverlayXattrPrefix::Trusted.as_str(),
        )
        .unwrap_err();
        assert_eq!(err.error(), Errno::EOPNOTSUPP);
    }

    #[ktest]
    fn present_strips_one_infix_keeps_prefix() {
        // A zero-capacity buffer is the size probe: reported = 19 + 1, nothing
        // written.
        let (total, written) =
            present_expect_ok(b"trusted.overlay.overlay.fsz\0", OverlayXattrPrefix::Trusted, 0);
        assert_eq!(total, 20);
        assert!(written.is_empty());
        // A large-enough buffer receives the presented bytes.
        let (total, written) =
            present_expect_ok(b"trusted.overlay.overlay.fsz\0", OverlayXattrPrefix::Trusted, 32);
        assert_eq!(total, 20);
        assert_eq!(written, b"trusted.overlay.fsz\0");
        // Exactly one infix segment is stripped per layer.
        let (_, written) = present_expect_ok(
            b"trusted.overlay.overlay.overlay.x\0",
            OverlayXattrPrefix::Trusted,
            64,
        );
        assert_eq!(written, b"trusted.overlay.overlay.x\0");
        // The prefix is kept at the boundary.
        let (_, written) =
            present_expect_ok(b"trusted.overlay.overlay.\0", OverlayXattrPrefix::Trusted, 64);
        assert_eq!(written, b"trusted.overlay.\0");
        // The same strip under the `userxattr` prefix.
        let (_, written) =
            present_expect_ok(b"user.overlay.overlay.fsz\0", OverlayXattrPrefix::User, 64);
        assert_eq!(written, b"user.overlay.fsz\0");
        // A mixed list: foreign entries pass through while the own escaped entry
        // is stripped.
        let (_, written) = present_expect_ok(
            b"user.plain\0trusted.overlay.overlay.fsz\0",
            OverlayXattrPrefix::Trusted,
            64,
        );
        assert_eq!(written, b"user.plain\0trusted.overlay.fsz\0");
    }

    #[ktest]
    fn present_hides_own_private_records() {
        for raw in [
            b"trusted.overlay.origin\0".as_slice(),
            b"trusted.overlay.opaque\0".as_slice(),
            b"trusted.overlay.whiteout\0".as_slice(),
            b"trusted.overlay.impure\0".as_slice(),
            b"trusted.overlay.uuid\0".as_slice(),
        ] {
            let (total, written) = present_expect_ok(raw, OverlayXattrPrefix::Trusted, 64);
            assert_eq!(total, 0);
            assert!(written.is_empty());
        }
        // Any own-prefix name without the infix is hidden.
        let (total, written) = present_expect_ok(
            b"trusted.overlay.no-infix-name\0",
            OverlayXattrPrefix::Trusted,
            64,
        );
        assert_eq!(total, 0);
        assert!(written.is_empty());
        // The infix without a trailing dot does not strip: hidden.
        let (total, written) =
            present_expect_ok(b"trusted.overlay.overlay\0", OverlayXattrPrefix::Trusted, 64);
        assert_eq!(total, 0);
        assert!(written.is_empty());
    }

    #[ktest]
    fn present_probe_and_erange_accounting() {
        // Size probe.
        let (total, written) = present_expect_ok(b"a\0bb\0", OverlayXattrPrefix::Trusted, 0);
        assert_eq!(total, 5);
        assert!(written.is_empty());
        // Exact fit.
        let (total, written) = present_expect_ok(b"a\0bb\0", OverlayXattrPrefix::Trusted, 5);
        assert_eq!(total, 5);
        assert_eq!(written, b"a\0bb\0");
        // Frozen boundary quirk: when the buffer fills exactly at an entry
        // boundary, the remaining entries are counted (probe semantics at
        // `avail == 0`); callers must use the reported length, not the buffer
        // contents.
        let (total, written) = present_expect_ok(b"a\0bb\0", OverlayXattrPrefix::Trusted, 2);
        assert_eq!(total, 5);
        assert_eq!(written, b"a\0");
        // Mid-stream overflow with `avail > 0`.
        present_expect_erange(b"a\0bbbb\0", OverlayXattrPrefix::Trusted, 3);
        // Empty list / empty entries skipped.
        let (total, written) = present_expect_ok(b"", OverlayXattrPrefix::Trusted, 64);
        assert_eq!(total, 0);
        assert!(written.is_empty());
        let (total, written) = present_expect_ok(b"\0\0", OverlayXattrPrefix::Trusted, 64);
        assert_eq!(total, 0);
        assert!(written.is_empty());
    }

    #[ktest]
    fn present_passes_foreign_and_non_utf8_through() {
        // Verbatim, including the non-UTF-8 entry.
        let (_, written) =
            present_expect_ok(b"user.plain\0\xff\xfe\0", OverlayXattrPrefix::Trusted, 64);
        assert_eq!(written, b"user.plain\0\xff\xfe\0");
        // Hiding requires a UTF-8 prefix match: non-UTF-8 own-looking names are
        // never hidden.
        let (_, written) =
            present_expect_ok(b"\xfftrusted.overlay.opaque\0", OverlayXattrPrefix::Trusted, 64);
        assert_eq!(written, b"\xfftrusted.overlay.opaque\0");
    }
}
