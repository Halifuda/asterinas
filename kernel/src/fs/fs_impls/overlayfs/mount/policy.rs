// SPDX-License-Identifier: MPL-2.0

//! Published mount-policy carriers (`P0-01`/`P0-02`/`P0-18`/`P1-19`/`P1-20`/`P2-11`).
//!
//! This module owns the immutable [`MountPolicy`] snapshot published by
//! [`OverlayFs`](super::superblock::OverlayFs), the creator-credential policy
//! ([`CreatorCredentialPolicy`], `P1-19`), the post-claim upper-filesystem
//! capability snapshot ([`UpperFilesystemCapabilities`], `P0-02`/`P2-11`/
//! `P1-25`/`P1-36`), and the minimal advisory write-access accounting
//! ([`WriteAccessAccounting`]/[`WriteAccessGuard`], `P1-20`). Sibling Mesos
//! read these published carriers only; they never re-create, copy ownership
//! of, or mutate them (spec §1 item 3).
//!
//! Construction happens once in `OverlayFs::new` (sibling `build.rs`): the
//! immutable snapshot is assembled by [`MountPolicy::assemble`] after every
//! fallible constituent exists (identity from step 5, capabilities from step
//! 7), and the write-access accounting is created only for genuinely writable
//! mounts (spec §4 invariant: `write_access` is `Some` iff
//! `is_effective_read_only` is false).
//!
//! Round-3 review item 1 (cross-meso consumption widening per the owner
//! rule): `MountPolicy` and `UpperFilesystemCapabilities` and the members the
//! `projection` tree consumes (`is_effective_read_only`,
//! `upper_capabilities`, `can_store_private_xattr`, `can_mknod_char`) are
//! published at the overlayfs ceiling.

use core::sync::atomic::{AtomicU64, Ordering};

use aster_rights::ReadDupOp;

use super::{
    claims::{OVERLAY_UUID_SIZE, OverlayUuid},
    options::UuidMode,
};
use crate::{
    fs::{
        file::{InodeMode, InodeType},
        utils::DirentVisitor,
        vfs::{
            file_system::{FileSystem, FsFlags},
            inode::{Inode, MknodType},
            path::is_dot_or_dotdot,
            xattr::XattrName,
        },
    },
    prelude::*,
    process::Credentials,
};

/// The private xattr name used by the read-only xattr-capability probe.
///
/// The probe reads the same private overlay namespace the unified identity is
/// persisted in (`trusted.overlay.uuid`, BC-1 §8; spec §1 RELY list:
/// "read/probe of `trusted.overlay.uuid`"). A backend that answers `ENODATA`
/// (no value yet) or returns the value supports the private namespace;
/// `EOPNOTSUPP` means it does not (fail-closed, spec §3.0.1 xattr evidence).
const PRIVATE_XATTR_PROBE_NAME: &str = "trusted.overlay.uuid";

/// Prefix of the uniquely-named temporary char-device probe entry created in
/// the workdir for the `can_mknod_char` probe (spec §4, revision 05).
const CHAR_DEVICE_PROBE_PREFIX: &str = ".overlay-char-device-probe-";

/// Prefix of the uniquely-named temporary file probe entry created in the
/// workdir for the d_type probe (wave-1 review items 5/23, consolidated
/// workdir temp-entry fix).
const D_TYPE_PROBE_PREFIX: &str = ".overlay-dtype-probe-";

/// Generates a uniquely-named workdir temp entry for a capability probe.
///
/// Shared by the d_type and char-device probes (wave-1 review round 2, item
/// 3): one `getrandom` + `format!` sequence instead of two copies (whitelist
/// Rule B: the exact logic is required at two sites within this module).
fn unique_temp_name(prefix: &str) -> String {
    let mut probe_bytes = [0u8; 8];
    crate::util::random::getrandom(&mut probe_bytes);
    format!("{}{:016x}", prefix, u64::from_le_bytes(probe_bytes))
}

/// The immutable, published mount policy snapshot (`P0-01`/`P0-02`/`P0-18`/
/// `P1-19`/`P1-20`/`P2-11`).
///
/// The snapshot is immutable after [`MountPolicy::assemble`] and is the only
/// representation of the frozen mount options/policy: `is_default_permissions`
/// is never duplicated or re-derived (not on `OverlayFs`), and `uuid` is
/// `Some` iff the unified identity is effective (spec §2 unified-identity
/// invariant / §4 invariants).
///
/// # Dev note (recorded deviation)
///
/// `#[derive(Debug)]` is dropped: the snapshot transitively holds
/// `Credentials<ReadDupOp>` (via [`CreatorCredentialPolicy`]), which has no
/// `Debug` impl (verified `kernel/src/process/credentials/mod.rs`); the spec
/// §4 shape hint explicitly allows dropping an unsatisfiable derive.
pub(in crate::fs::fs_impls::overlayfs) struct MountPolicy {
    /// Effective read-only state (`P0-18`), frozen before any claim is taken.
    is_effective_read_only: bool,
    /// The UUID/fsid mode (`P2-11`).
    uuid_mode: UuidMode,
    /// The unified overlay identity; `Some` iff effective (`P2-11`).
    uuid: Option<OverlayUuid>,
    /// The stashed creator-credential policy (`P1-19`).
    credential_policy: CreatorCredentialPolicy,
    /// The advisory write-access accounting; `Some` iff writable (`P1-20`).
    write_access: Option<WriteAccessAccounting>,
    /// The post-claim upper-filesystem capability snapshot (`P0-02`).
    upper_capabilities: Option<UpperFilesystemCapabilities>,
    /// Whether the mount was created with the `default_permissions` option
    /// (`P0-01`; the frozen option value meso-05's stage-B skip consumes).
    is_default_permissions: bool,
}

impl MountPolicy {
    /// Assembles the immutable policy snapshot.
    ///
    /// The single assembly point (spec §4); the seven parameters are exactly
    /// the published snapshot's constituents — the one deliberate >3-param
    /// exception in the complexity baseline. Called once from `OverlayFs::new`
    /// (sibling `build.rs`) after all fallible constituents exist.
    pub(super) fn assemble(
        is_effective_read_only: bool,
        credential_policy: CreatorCredentialPolicy,
        uuid_mode: UuidMode,
        uuid: Option<OverlayUuid>,
        upper_capabilities: Option<UpperFilesystemCapabilities>,
        write_access: Option<WriteAccessAccounting>,
        is_default_permissions: bool,
    ) -> Self {
        Self {
            is_effective_read_only,
            uuid_mode,
            uuid,
            credential_policy,
            write_access,
            upper_capabilities,
            is_default_permissions,
        }
    }

    /// Reports the effective read-only state (`P0-18`).
    ///
    /// Widened to the overlayfs ceiling (round-3 review item 1): consumed by
    /// `OverlayInode::read_only_gate` from the `projection` tree.
    pub(in crate::fs::fs_impls::overlayfs) fn is_effective_read_only(&self) -> bool {
        self.is_effective_read_only
    }

    /// Reports the frozen `default_permissions` option value (`P0-01`).
    ///
    /// The exact accessor meso-05 consumes as
    /// `self.fs_arc()?.policy().is_default_permissions()` in its stage-B skip
    /// (BC-5 §49); it reports the frozen option value only — the skip
    /// semantics are meso-05's (spec §1 item 4).
    pub(in crate::fs::fs_impls::overlayfs) fn is_default_permissions(&self) -> bool {
        self.is_default_permissions
    }

    /// Returns the frozen UUID/fsid mode (`P2-11`).
    #[expect(
        dead_code,
        reason = "frozen published accessor (spec §4); consumed by sibling mesos once they land"
    )]
    pub(super) fn uuid_mode(&self) -> UuidMode {
        self.uuid_mode
    }

    /// Returns the effective unified overlay identity, if any (`P2-11`).
    ///
    /// `Some` iff the identity is effective; the persisted value is never
    /// changed during the mount lifetime (spec §2 unified-identity invariant).
    pub(super) fn uuid(&self) -> Option<&OverlayUuid> {
        self.uuid.as_ref()
    }

    /// Returns the stashed creator-credential policy (`P1-19`).
    ///
    /// Wave-4 repair item 7: the `#[expect(dead_code)]` marker is removed —
    /// the accessor is live code, consumed by the landed meso-05 entries
    /// (`metadata_security/mod.rs` delegation and `permission.rs` stage B).
    pub(in crate::fs::fs_impls::overlayfs) fn credential_policy(&self) -> &CreatorCredentialPolicy {
        &self.credential_policy
    }

    /// Returns the advisory write-access accounting, if this is a writable
    /// mount (`P1-20`).
    #[expect(
        dead_code,
        reason = "frozen published accessor (spec §4); consumed by meso write-access consumers once they land"
    )]
    pub(super) fn write_access(&self) -> Option<&WriteAccessAccounting> {
        self.write_access.as_ref()
    }

    /// Returns the post-claim upper-filesystem capability snapshot, if this
    /// is a writable mount (`P0-02`).
    #[expect(
        dead_code,
        reason = "frozen published accessor (spec §4); consumed by the P1-07 store seam (lower_id.rs) and later by meso-06 `WhiteoutRepresentation`"
    )]
    pub(in crate::fs::fs_impls::overlayfs) fn upper_capabilities(
        &self,
    ) -> Option<&UpperFilesystemCapabilities> {
        self.upper_capabilities.as_ref()
    }
}

/// The creator-credential policy of an overlay mount (`P1-19`).
///
/// Stashes the mounting thread's credentials once, at construction, and
/// publishes the scoped-override contract ([`CreatorCredentialPolicy::with_creator_credentials_fn`])
/// that sibling Mesos use for underlying VFS calls. The credential snapshot is
/// immutable after construction (spec §2 immutability invariant).
///
/// # Dev note (recorded deviation)
///
/// `#[derive(Debug)]` is dropped: `Credentials<ReadDupOp>` has no `Debug` impl
/// (verified `kernel/src/process/credentials/mod.rs`); the spec §4 shape hint
/// explicitly allows dropping an unsatisfiable derive.
pub(in crate::fs::fs_impls::overlayfs) struct CreatorCredentialPolicy {
    /// The stashed creator credentials (`P1-19`), taken once at construction
    /// via `fs_creation_ctx.task_ctx().posix_thread.credentials_dup()`.
    snapshot: Credentials<ReadDupOp>,
    /// The credential source; [`CredentialSource::Creator`] in this wave
    /// (`P3-07` insertion point: add `Caller`).
    source: CredentialSource,
}

impl CreatorCredentialPolicy {
    /// Creates the policy from the mounting thread's credential snapshot.
    ///
    /// `build.rs` takes the snapshot once at construction
    /// (`ctx.task_ctx().posix_thread.credentials_dup()`, spec §3.2 step 4).
    pub(super) fn new(snapshot: Credentials<ReadDupOp>) -> Self {
        Self {
            snapshot,
            source: CredentialSource::Creator,
        }
    }

    /// Returns the stashed creator credentials.
    #[expect(
        dead_code,
        reason = "frozen creator-credential contract (spec §4, P1-19); consumed by meso-04+ once they land"
    )]
    pub(in crate::fs::fs_impls::overlayfs) fn snapshot(&self) -> &Credentials<ReadDupOp> {
        &self.snapshot
    }

    /// Returns the credential source (closed set: `Creator` this wave).
    #[expect(
        dead_code,
        reason = "frozen creator-credential contract (spec §4, P1-19); consumed by meso-04+ once they land"
    )]
    pub(in crate::fs::fs_impls::overlayfs) fn source(&self) -> CredentialSource {
        self.source
    }

    /// Runs `operation_fn` under the stashed creator credentials.
    ///
    /// The scoped credential-swap mechanism is a recorded VFS dependency
    /// (spec §3.0.5 item 4): Asterinas `PosixThread` exposes
    /// `credentials()`/`credentials_dup()`/`credentials_mut()` but no scoped
    /// "run with stashed credentials" API, and `Inode::check_permission` uses
    /// `Task::current()` implicitly. Until that seam lands, `operation_fn`
    /// runs with the caller's current credentials and the stashed snapshot is
    /// published for sibling Mesos but cannot be installed (temporary seam,
    /// recorded in the Creator report); no frozen signature is changed.
    ///
    /// Wave-4 repair item 7: the `#[expect(dead_code)]` marker is removed —
    /// the seam is live code, consumed by the landed meso-05 delegation
    /// helper (`metadata_security/mod.rs`) and the real permission stage.
    pub(in crate::fs::fs_impls::overlayfs) fn with_creator_credentials_fn<T>(
        &self,
        operation_fn: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        operation_fn()
    }
}

/// The source of the credentials used for underlying overlayfs calls (`P1-19`).
///
/// Closed set this wave: the mount creator's credentials are always used
/// (`P3-07` adds `Caller` under an explicit scope decision).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::fs::fs_impls::overlayfs) enum CredentialSource {
    /// The credentials of the task that created the mount.
    Creator,
}

/// The post-claim upper-filesystem capability snapshot (`P0-02`/`P2-11`/
/// `P1-25`/`P1-36`).
///
/// Immutable after construction; `can_mknod_char` and `can_store_private_xattr`
/// are single-representation probe results that consumers (e.g., meso-06's
/// `WhiteoutRepresentation` derivation) never re-probe or re-derive (spec §1
/// item 5 / §4 invariants).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::fs::fs_impls::overlayfs) struct UpperFilesystemCapabilities {
    /// Whether the upper can store private overlay xattrs (`trusted.`/`user.`
    /// namespace; `P0-02`/`P2-11` gate).
    can_store_private_xattr: bool,
    /// Whether the upper reports directory entry types (`d_type` in readdir;
    /// `P0-02` hard gate).
    can_report_directory_type: bool,
    /// Whether the workdir supports the classic whiteout char device `0:0`
    /// (revision 05; `P1-25`/`P1-36` whiteout-form gate).
    can_mknod_char: bool,
}

impl UpperFilesystemCapabilities {
    /// Probes the upper/workdir capabilities post-claim (spec §3.2 step 7).
    ///
    /// Writable mounts only, sleep-capable construction context. The xattr
    /// probe (`get_xattr` on the private overlay namespace) is read-only on
    /// the upper; the d_type probe creates a uniquely-named temporary file in
    /// the workdir, scans the workdir until exhausted, and removes the temp
    /// (wave-1 review items 5/23 — a workdir entry guarantees a non-vacuous
    /// probe); the `can_mknod_char` probe creates a uniquely-named temporary
    /// char device (`Inode::mknod`, `MknodType::CharDevice(0)`) in the workdir
    /// and removes it on success and failure — no workdir residue (spec §3.3
    /// Hazard 1). Each probe is a small per-capability helper and the temp
    /// entry names share one [`unique_temp_name`] generator (wave-1 review
    /// round 2, item 3).
    pub(super) fn probe(
        upper_inode: &Arc<dyn Inode>,
        workdir_inode: &Arc<dyn Inode>,
    ) -> Result<Self> {
        let can_store_private_xattr = Self::probe_private_xattr(upper_inode)?;
        let can_report_directory_type = Self::probe_d_type(workdir_inode)?;
        let can_mknod_char = Self::probe_mknod_char(workdir_inode)?;
        Ok(Self {
            can_store_private_xattr,
            can_report_directory_type,
            can_mknod_char,
        })
    }

    /// Probes whether the upper stores private overlay xattrs (`P0-02`/`P2-11`
    /// gate).
    ///
    /// Read-only on the upper. A backend that answers `ENODATA` (no value yet),
    /// `ERANGE` (a value is present but larger than the probe buffer — itself
    /// positive evidence the private namespace is stored), or returns the
    /// value supports the namespace; `EOPNOTSUPP` means it does not
    /// (fail-closed, spec §3.0.1 xattr evidence). Any other error is
    /// propagated. `ERANGE` maps to supported so `UuidMode::Auto` degrades
    /// instead of failing on an over-long foreign value (wave-1 review round
    /// 2, item 1).
    fn probe_private_xattr(upper_inode: &Arc<dyn Inode>) -> Result<bool> {
        let name = XattrName::try_from_full_name(PRIVATE_XATTR_PROBE_NAME).ok_or_else(|| {
            Error::with_message(Errno::EINVAL, "invalid overlay xattr probe name")
        })?;
        // The probe buffer is sized at the persisted `trusted.overlay.uuid`
        // value length (`OVERLAY_UUID_SIZE`, 8 bytes): a backend that
        // returns `ERANGE` for a short read (ramfs, ext2) must be counted
        // as supporting the private namespace, not fail the mount
        // (wave-1 review item 1 — a 1-byte buffer turned `UuidMode::Auto`
        // re-mounts of an upper with a persisted uuid into a
        // deterministic `ERANGE` failure).
        let mut value = [0u8; OVERLAY_UUID_SIZE];
        let mut writer = VmWriter::from(&mut value).to_fallible();
        match upper_inode.get_xattr(name, &mut writer) {
            Ok(_) => true,
            Err(err) if err.error() == Errno::ENODATA => true,
            Err(err) if err.error() == Errno::ERANGE => true,
            Err(err) if err.error() == Errno::EOPNOTSUPP => false,
            Err(err) => return Err(err),
        }
    }

    /// Probes whether the upper reports directory entry types (`P0-02` hard
    /// gate).
    ///
    /// Wave-1 review items 5/23, consolidated workdir temp-entry fix: probe a
    /// directory guaranteed to contain at least one non-dot entry instead of
    /// the usually-empty upper root. A uniquely-named temp file is created in
    /// the workdir (the same underlying filesystem as the upper, enforced by
    /// the `container_dev_id` check in `validate_pair`), the workdir is
    /// scanned until exhausted, and the temp is removed — an empty upper root
    /// can no longer make the gate pass vacuously, and `InodeType::Unknown`
    /// on any non-dot entry is the concrete evidence of a backend without
    /// `d_type` (fail-closed). Residue cleanup is best-effort on the failure
    /// path (spec §3.3 Hazard 1 "no workdir residue").
    fn probe_d_type(workdir_inode: &Arc<dyn Inode>) -> Result<bool> {
        let d_type_probe_name = unique_temp_name(D_TYPE_PROBE_PREFIX);
        workdir_inode.create(&d_type_probe_name, InodeType::File, InodeMode::empty())?;
        let mut d_type_probe = DTypeProbeVisitor::new();
        let mut offset = 0;
        let d_type_scan_result = loop {
            match workdir_inode.readdir_at(offset, &mut d_type_probe) {
                Ok(0) => break Ok(()),
                Ok(visited) => offset += visited,
                Err(err) => break Err(err),
            }
        };
        match d_type_scan_result {
            Ok(()) => {
                workdir_inode.unlink(&d_type_probe_name)?;
                Ok(!d_type_probe.saw_unknown_non_dot)
            }
            Err(err) => {
                let _ = workdir_inode.unlink(&d_type_probe_name);
                Err(err)
            }
        }
    }

    /// Probes whether the workdir supports the classic whiteout char device
    /// `0:0` (revision 05; `P1-25`/`P1-36` whiteout-form gate).
    ///
    /// The workdir hosts a uniquely-named temporary char device `0:0`;
    /// `EOPNOTSUPP` means no classic-whiteout form. The temp is removed inline
    /// on success; only an `unlink` failure after a successful `mknod` can
    /// leave residue, which fails the mount closed (the explicit residue
    /// cleanup is the `P3-09` insertion point, spec §2.4).
    fn probe_mknod_char(workdir_inode: &Arc<dyn Inode>) -> Result<bool> {
        let probe_name = unique_temp_name(CHAR_DEVICE_PROBE_PREFIX);
        match workdir_inode.mknod(&probe_name, InodeMode::empty(), MknodType::CharDevice(0)) {
            Ok(_) => {
                workdir_inode.unlink(&probe_name)?;
                Ok(true)
            }
            Err(err) if err.error() == Errno::EOPNOTSUPP => Ok(false),
            Err(err) => Err(err),
        }
    }

    /// Reports whether the upper can store private overlay xattrs.
    ///
    /// Widened to the overlayfs ceiling (round-3 review item 1): consumed by
    /// the P1-07 store seam (`projection/lower_id.rs`) from the `projection`
    /// tree.
    pub(in crate::fs::fs_impls::overlayfs) fn can_store_private_xattr(&self) -> bool {
        self.can_store_private_xattr
    }

    /// Reports whether the upper reports directory entry types.
    pub(super) fn can_report_directory_type(&self) -> bool {
        self.can_report_directory_type
    }

    /// Reports whether the workdir supports the classic whiteout char device
    /// `0:0` (revision 05).
    ///
    /// Widened to the overlayfs ceiling (round-3 review item 1, as consumed).
    pub(in crate::fs::fs_impls::overlayfs) fn can_mknod_char(&self) -> bool {
        self.can_mknod_char
    }
}

/// A [`DirentVisitor`] that records whether any non-dot entry reports
/// `InodeType::Unknown`.
///
/// Mandated by the `readdir_at` interface (whitelist Rule C: no existing
/// `DirentVisitor` implementation captures entry types), the visitor is the
/// localized shape for the read-only d_type probe of
/// [`UpperFilesystemCapabilities::probe`].
struct DTypeProbeVisitor {
    /// Whether any non-dot entry reported an unknown type.
    saw_unknown_non_dot: bool,
}

impl DTypeProbeVisitor {
    fn new() -> Self {
        Self {
            saw_unknown_non_dot: false,
        }
    }
}

impl DirentVisitor for DTypeProbeVisitor {
    fn visit(&mut self, name: &str, _ino: u64, type_: InodeType, _offset: usize) -> Result<()> {
        if !is_dot_or_dotdot(name) && type_ == InodeType::Unknown {
            self.saw_unknown_non_dot = true;
        }
        Ok(())
    }
}

/// The minimal advisory write-access accounting of a writable overlay
/// (`P1-20`).
///
/// A saturating mount-local user counter with an RAII guard
/// ([`WriteAccessGuard`]); the counter never gates underlying I/O and is not
/// a second ownership source (spec §4 invariants). Frozen as minimal because
/// Asterinas has no VFS superblock freeze/`want_write` API (recorded
/// limitation, spec §3.0.5 item 5a).
#[derive(Debug)]
pub(super) struct WriteAccessAccounting {
    /// The accounted upper filesystem (defensive `EROFS` gate).
    upper_fs: Arc<dyn FileSystem>,
    /// The saturating active-user count (never wraps).
    active_write_users: AtomicU64,
}

impl WriteAccessAccounting {
    /// Creates the accounting for a writable upper filesystem.
    ///
    /// Called once from `OverlayFs::new` for genuinely writable mounts only
    /// (spec §3.2 step 4 / §4 invariant: `write_access` is `Some` iff
    /// `is_effective_read_only` is false).
    pub(super) fn new(upper_fs: Arc<dyn FileSystem>) -> Self {
        Self {
            upper_fs,
            active_write_users: AtomicU64::new(0),
        }
    }

    /// Takes one advisory write-access user slot.
    ///
    /// Defensively fails with `EROFS` when the accounted upper now reports
    /// read-only (spec §2 Case 9); otherwise the saturating counter is
    /// incremented and the RAII guard returned. The increment is a
    /// non-blocking single-word atomic update that never wraps (spec §4
    /// invariant).
    #[expect(
        dead_code,
        reason = "frozen P1-20 advisory guard intake (spec §4); consumed by write-access consumers once they land"
    )]
    pub(super) fn try_get_write_access(&self) -> Result<WriteAccessGuard<'_>> {
        if self.upper_fs.flags().contains(FsFlags::RDONLY) {
            return Err(Error::new(Errno::EROFS));
        }
        let _ =
            self.active_write_users
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
                    Some(count.saturating_add(1))
                });
        Ok(WriteAccessGuard { accounting: self })
    }

    /// Reports whether any write-access user is currently counted.
    #[expect(
        dead_code,
        reason = "frozen P1-20 advisory reporting accessor (spec §4); consumed by sibling mesos once they land"
    )]
    pub(super) fn has_active_write_users(&self) -> bool {
        self.active_write_users.load(Ordering::Relaxed) > 0
    }

    /// Returns the current advisory active-user count.
    #[expect(
        dead_code,
        reason = "frozen P1-20 advisory reporting accessor (spec §4); consumed by sibling mesos once they land"
    )]
    pub(super) fn active_write_user_count(&self) -> u64 {
        self.active_write_users.load(Ordering::Relaxed)
    }
}

/// A short-lived RAII borrow of one advisory write-access slot (`P1-20`).
///
/// `Drop` decrements the saturating counter; the guard never gates underlying
/// I/O (spec §2 Case 9).
#[derive(Debug)]
#[expect(
    dead_code,
    reason = "frozen P1-20 RAII carrier published to sibling mesos (spec §4); constructed only through the deferred try_get_write_access"
)]
pub(super) struct WriteAccessGuard<'a> {
    /// The accounting this guard borrows; `Drop` decrements it.
    accounting: &'a WriteAccessAccounting,
}

impl Drop for WriteAccessGuard<'_> {
    fn drop(&mut self) {
        let _ = self.accounting.active_write_users.fetch_update(
            Ordering::Relaxed,
            Ordering::Relaxed,
            |count| Some(count.saturating_sub(1)),
        );
    }
}
