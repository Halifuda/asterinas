// SPDX-License-Identifier: MPL-2.0

#![short_vis_path::add(overlayfs)]
//! Mount option parsing for overlayfs.
//!
//! This module validates the mount option string into an
//! [`MountOptions`] construction input. The recognized keys are the string
//! keys `lowerdir`, `lowerdir+`, `upperdir`, `workdir`, `uuid`, and `xino`,
//! the enum keys `redirect_dir`, `index`, `nfs_export`, `metacopy`,
//! `verity`, and `fsync`, and the valueless keys `default_permissions`,
//! `userxattr`, `volatile`, and `nooverride_creds`; unknown keys fail with
//! `EINVAL` before any layer state is created, as do `datadir+` and
//! `override_creds`. A key whose implementing feature is absent is accepted
//! as raw intent and disclosed as a one-shot mount-time degrade by the
//! verify phase.
//!
//! ## References
//!
//! - <https://elixir.bootlin.com/linux/v7.0/source/Documentation/filesystems/overlayfs.rst#L350-L364>
//!   (Linux stacks colon-separated lowerdirs with the first entry topmost)

use super::super::policy::{UuidMode, XinoMode};
use crate::{fs::vfs::file_system::FsFlags, prelude::*};

/// The `redirect_dir=` mode (parse intent). Upstream constant table
/// `ovl_parameter_redirect_dir` (Linux `fs/overlayfs/params.c:107-113`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RedirectDirMode {
    /// Upstream remaps `off` per the module param `redirect_always_follow`
    /// (`params.c:659-663`); local has no module param, so `off` degrades to
    /// nofollow like the other non-nofollow values.
    Off,
    /// `follow`: honor recorded redirects when the redirect_dir feature is
    /// off.
    Follow,
    /// `nofollow`: never follow recorded redirects.
    NoFollow,
    /// `on`: record redirects on directory copy-up and follow them.
    On,
}

/// The `verity=` mode (parse intent). Upstream `ovl_parameter_verity`
/// (Linux `fs/overlayfs/params.c:127-132`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum VerityMode {
    /// Never generate or check metacopy digests (upstream default; the local
    /// effective state for every value).
    Off,
    /// Check digests when present; set them on metacopy generation.
    On,
    /// Like [`VerityMode::On`], plus metacopy without a digest is rejected
    /// (full copy-up).
    Require,
}

/// The `fsync=` mode (parse intent). Upstream `ovl_parameter_fsync`
/// (Linux `fs/overlayfs/params.c:144-149`); the valueless `volatile` option
/// is an upstream alias for `fsync=volatile` (`params.c:690-692`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FsyncMode {
    /// Prefer performance over durability (volatile).
    Volatile,
    /// fsync upper data before completing data copy-up (upstream default;
    /// honored by the existing `sync_all` before the rename in copy-up).
    Auto,
    /// fsync upper data and metadata/directories before completing any
    /// copy-up (not implementable locally this round).
    Strict,
}

/// Validated construction input for an overlay mount.
///
/// The struct is constructed once by [`MountOptions::parse`], consumed once
/// by `OverlayFs::new`, and dropped before the filesystem is published: it
/// carries raw mount intent only, while every effective per-mount decision
/// lives on `MountPolicy` (and the identity state derived beside it). No
/// runtime module reads this struct.
#[derive(Debug)]
pub(super) struct MountOptions {
    /// Lower layer paths in option order; the first option is the topmost.
    pub(super) lower_dirs: Vec<String>,
    /// Upper layer path; `None` means a read-only overlay.
    pub(super) upper_dir: Option<String>,
    /// Work directory path; `Some` iff `upper_dir` is `Some`.
    pub(super) work_dir: Option<String>,
    pub(super) is_forced_read_only: bool,
    pub(super) is_default_permissions: bool,
    /// The `userxattr` mode: the selected private-xattr prefix is
    /// `user.overlay.` instead of the default `trusted.overlay.`.
    pub(super) is_userxattr: bool,
    /// The UUID persistence mode; `None` means [`UuidMode::Auto`].
    pub(super) uuid_mode: Option<UuidMode>,
    /// The `xino=` mode; `None` means [`XinoMode::Auto`].
    pub(super) xino_mode: Option<XinoMode>,
    /// The raw `redirect_dir=` intent. `Some(_)` records an explicitly
    /// requested mode; recorded directory redirects are not implemented, so
    /// every value but [`RedirectDirMode::NoFollow`] is disclosed as a
    /// one-shot degrade by the parse verify phase.
    redirect_dir: Option<RedirectDirMode>,
    /// The raw `index=` intent. `Some(_)` records an explicitly requested
    /// state; no inode index is maintained, so `on` is disclosed as a
    /// one-shot degrade by the parse verify phase.
    index: Option<bool>,
    /// The raw `nfs_export=` intent. `Some(_)` records an explicitly
    /// requested state; no export file handles are encoded, so `on` is
    /// disclosed as a one-shot degrade by the parse verify phase.
    nfs_export: Option<bool>,
    /// The raw `metacopy=` intent. `Some(_)` records an explicitly requested
    /// state; copy-up always copies data, so `on` is disclosed as a one-shot
    /// degrade by the parse verify phase.
    metacopy: Option<bool>,
    /// The raw `verity=` intent. `Some(_)` records an explicitly requested
    /// mode; fs-verity digests are not enforced, so every value but
    /// [`VerityMode::Off`] is disclosed as a one-shot degrade by the parse
    /// verify phase.
    verity: Option<VerityMode>,
    /// The raw `fsync=` intent; the valueless `volatile` option aliases into
    /// `Some(FsyncMode::Volatile)`. Only [`FsyncMode::Auto`] is honored
    /// locally.
    fsync_mode: Option<FsyncMode>,
}

impl MountOptions {
    /// Parses the comma-separated mount option string plus the mount flags
    /// into a validated [`MountOptions`].
    ///
    /// Parsing contract (all violations fail with `EINVAL`):
    ///
    /// * Key/value and splitting: the string is split on `,`, empty entries
    ///   are skipped, and the first `=` of an entry separates the key from a
    ///   verbatim value (no quoting, no backslash unescaping); a bare `key`
    ///   is a valueless option accepted only for `default_permissions`,
    ///   `userxattr`, `volatile`, and `nooverride_creds`.
    /// * Key domains and repetition: `lowerdir` is required and is a
    ///   non-empty, colon-separated layer list (first path topmost) with no
    ///   empty layer; each `lowerdir+` occurrence takes one verbatim path
    ///   (colons are not split) appended as an extra lower layer and cannot
    ///   be combined with `lowerdir`; `upperdir`/`workdir` each take a
    ///   single non-empty path and must both be present or both be absent;
    ///   `uuid` accepts only `off`/`null`/`on`/`auto`, `xino` accepts only
    ///   `off`/`auto`/`on`, `redirect_dir` accepts only
    ///   `off`/`follow`/`nofollow`/`on`, `index`/`nfs_export`/`metacopy`
    ///   accept only `on`/`off`, `verity` accepts only `off`/`on`/`require`,
    ///   `fsync` accepts only `volatile`/`auto`/`strict`, and
    ///   `default_permissions`/`userxattr`/`volatile`/`nooverride_creds`
    ///   take no value; every key may appear at most once, with `volatile`
    ///   and `fsync` sharing one duplicate slot.
    /// * Cross-key rules: explicit option conflicts fail with `EINVAL`
    ///   (enforced by the verify phase); an explicitly requested feature
    ///   that is not implemented is accepted as raw intent and disclosed as
    ///   a one-shot mount-time degrade.
    /// * Required-value constraint: `None` (no option string) fails like a
    ///   missing `lowerdir`.
    pub(super) fn parse(args: Option<&str>, fs_flags: FsFlags) -> Result<Self> {
        let mut lower_dirs = Vec::new();
        let mut upper_dir = None;
        let mut work_dir = None;
        let mut is_default_permissions = false;
        let mut is_userxattr = false;
        let mut uuid_mode = None;
        let mut xino_mode = None;
        let mut redirect_dir = None;
        let mut index = None;
        let mut nfs_export = None;
        let mut metacopy = None;
        let mut verity = None;
        let mut fsync_mode = None;
        // Both `lowerdir` forms populate `lower_dirs` jointly, so the
        // duplicate and mixing rules need dedicated per-key flags.
        let mut is_lowerdir_seen = false;
        let mut is_lowerdir_plus_seen = false;
        let mut is_nooverride_creds_seen = false;

        let Some(args) = args else {
            return_errno_with_message!(
                Errno::EINVAL,
                "the `lowerdir` mount option must be specified"
            );
        };
        for entry in args.split(',') {
            if entry.is_empty() {
                continue;
            }
            let (key_name, value) = match entry.split_once('=') {
                Some((key_name, value)) => (key_name, Some(value)),
                None => (entry, None),
            };
            match key_name {
                "lowerdir" | "lowerdir+" | "upperdir" | "workdir" | "uuid" | "xino"
                | "redirect_dir" | "index" | "nfs_export" | "metacopy" | "verity" | "fsync" => {
                    let Some(value) = value else {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "the `{key_name}` mount option requires a value"
                        );
                    };
                    match key_name {
                        "lowerdir" => {
                            // The first lowerdir path is the topmost layer.
                            if is_lowerdir_seen {
                                return_errno_with_message!(
                                    Errno::EINVAL,
                                    "duplicate overlay mount option `lowerdir`"
                                );
                            }
                            if is_lowerdir_plus_seen {
                                return_errno_with_message!(
                                    Errno::EINVAL,
                                    "the `lowerdir+` mount option cannot be combined with `lowerdir`"
                                );
                            }
                            Self::require_non_empty_value(
                                value,
                                "the `lowerdir` mount option requires a non-empty value",
                            )?;
                            lower_dirs = value.split(':').map(str::to_string).collect();
                            if lower_dirs.iter().any(|lower_dir| lower_dir.is_empty()) {
                                return_errno_with_message!(
                                    Errno::EINVAL,
                                    "the `lowerdir` value contains an empty layer path"
                                );
                            }
                            is_lowerdir_seen = true;
                        }
                        "lowerdir+" => {
                            // Each occurrence appends one verbatim lower
                            // path: colons are not split and no unescaping
                            // is applied.
                            Self::require_non_empty_value(
                                value,
                                "the `lowerdir+` mount option requires a non-empty value",
                            )?;
                            if is_lowerdir_seen {
                                return_errno_with_message!(
                                    Errno::EINVAL,
                                    "the `lowerdir+` mount option cannot be combined with `lowerdir`"
                                );
                            }
                            lower_dirs.push(value.to_string());
                            is_lowerdir_plus_seen = true;
                        }
                        "upperdir" => {
                            // The upperdir is the writable top layer on writable mounts.
                            if upper_dir.is_some() {
                                return_errno_with_message!(
                                    Errno::EINVAL,
                                    "duplicate overlay mount option `upperdir`"
                                );
                            }
                            Self::require_non_empty_value(
                                value,
                                "the `upperdir` mount option requires a non-empty value",
                            )?;
                            upper_dir = Some(value.to_string());
                        }
                        "workdir" => {
                            // The workdir stores the staging workspace for copy-up/remove.
                            if work_dir.is_some() {
                                return_errno_with_message!(
                                    Errno::EINVAL,
                                    "duplicate overlay mount option `workdir`"
                                );
                            }
                            Self::require_non_empty_value(
                                value,
                                "the `workdir` mount option requires a non-empty value",
                            )?;
                            work_dir = Some(value.to_string());
                        }
                        "uuid" => {
                            // The UUID mode controls overlay uuid/fsid behavior.
                            if uuid_mode.is_some() {
                                return_errno_with_message!(
                                    Errno::EINVAL,
                                    "duplicate overlay mount option `uuid`"
                                );
                            }
                            uuid_mode = Some(match value {
                                "off" => UuidMode::Off,
                                "null" => UuidMode::Null,
                                "on" => UuidMode::On,
                                "auto" => UuidMode::Auto,
                                _ => {
                                    return_errno_with_message!(
                                        Errno::EINVAL,
                                        "invalid `uuid` mount option value"
                                    );
                                }
                            });
                        }
                        "xino" => {
                            // The xino mode controls dev/ino projection encoding.
                            if xino_mode.is_some() {
                                return_errno_with_message!(
                                    Errno::EINVAL,
                                    "duplicate overlay mount option `xino`"
                                );
                            }
                            xino_mode = Some(match value {
                                "off" => XinoMode::Off,
                                "auto" => XinoMode::Auto,
                                "on" => XinoMode::On,
                                _ => {
                                    return_errno_with_message!(
                                        Errno::EINVAL,
                                        "invalid `xino` mount option value"
                                    );
                                }
                            });
                        }
                        "redirect_dir" => {
                            // The raw mode is kept for the verify phase: the
                            // recorded-redirect feature is absent, so every
                            // value but `nofollow` degrades there.
                            if redirect_dir.is_some() {
                                return_errno_with_message!(
                                    Errno::EINVAL,
                                    "duplicate overlay mount option `redirect_dir`"
                                );
                            }
                            redirect_dir = Some(match value {
                                "off" => RedirectDirMode::Off,
                                "follow" => RedirectDirMode::Follow,
                                "nofollow" => RedirectDirMode::NoFollow,
                                "on" => RedirectDirMode::On,
                                _ => {
                                    return_errno_with_message!(
                                        Errno::EINVAL,
                                        "invalid `redirect_dir` mount option value"
                                    );
                                }
                            });
                        }
                        "index" => {
                            // The raw intent is kept for the verify phase: no
                            // inode index is maintained, so `on` degrades there.
                            if index.is_some() {
                                return_errno_with_message!(
                                    Errno::EINVAL,
                                    "duplicate overlay mount option `index`"
                                );
                            }
                            index = Some(match value {
                                "on" => true,
                                "off" => false,
                                _ => {
                                    return_errno_with_message!(
                                        Errno::EINVAL,
                                        "invalid `index` mount option value"
                                    );
                                }
                            });
                        }
                        "nfs_export" => {
                            // The raw intent is kept for the verify phase: no
                            // export file handles are encoded, so `on`
                            // degrades there.
                            if nfs_export.is_some() {
                                return_errno_with_message!(
                                    Errno::EINVAL,
                                    "duplicate overlay mount option `nfs_export`"
                                );
                            }
                            nfs_export = Some(match value {
                                "on" => true,
                                "off" => false,
                                _ => {
                                    return_errno_with_message!(
                                        Errno::EINVAL,
                                        "invalid `nfs_export` mount option value"
                                    );
                                }
                            });
                        }
                        "metacopy" => {
                            // The raw intent is kept for the verify phase:
                            // copy-up always copies data, so `on` degrades there.
                            if metacopy.is_some() {
                                return_errno_with_message!(
                                    Errno::EINVAL,
                                    "duplicate overlay mount option `metacopy`"
                                );
                            }
                            metacopy = Some(match value {
                                "on" => true,
                                "off" => false,
                                _ => {
                                    return_errno_with_message!(
                                        Errno::EINVAL,
                                        "invalid `metacopy` mount option value"
                                    );
                                }
                            });
                        }
                        "verity" => {
                            // The raw intent is kept for the verify phase:
                            // fs-verity digests are not enforced, so every
                            // value but `off` degrades there.
                            if verity.is_some() {
                                return_errno_with_message!(
                                    Errno::EINVAL,
                                    "duplicate overlay mount option `verity`"
                                );
                            }
                            verity = Some(match value {
                                "off" => VerityMode::Off,
                                "on" => VerityMode::On,
                                "require" => VerityMode::Require,
                                _ => {
                                    return_errno_with_message!(
                                        Errno::EINVAL,
                                        "invalid `verity` mount option value"
                                    );
                                }
                            });
                        }
                        "fsync" => {
                            // Only `auto` is honored: data copy-up already
                            // syncs before publication. The valueless
                            // `volatile` alias shares this duplicate slot.
                            if fsync_mode.is_some() {
                                return_errno_with_message!(
                                    Errno::EINVAL,
                                    "duplicate overlay mount option `fsync`"
                                );
                            }
                            fsync_mode = Some(match value {
                                "volatile" => FsyncMode::Volatile,
                                "auto" => FsyncMode::Auto,
                                "strict" => FsyncMode::Strict,
                                _ => {
                                    return_errno_with_message!(
                                        Errno::EINVAL,
                                        "invalid `fsync` mount option value"
                                    );
                                }
                            });
                        }
                        _ => unreachable!("key_name was filtered above"),
                    }
                }
                "default_permissions" => {
                    // This option takes no value.
                    if is_default_permissions {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "duplicate overlay mount option `default_permissions`"
                        );
                    }
                    if value.is_some() {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "the `default_permissions` mount option does not take a value"
                        );
                    }
                    is_default_permissions = true;
                }
                "userxattr" => {
                    // This option takes no value. It selects `user.overlay.`
                    // as the private-xattr prefix (the unprivileged
                    // workaround). Its explicit conflicts with `redirect_dir`
                    // (anything but `nofollow`) and with `metacopy=on` are
                    // enforced by the verify phase (Linux
                    // `fs/overlayfs/params.c:988-1008`).
                    if is_userxattr {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "duplicate overlay mount option `userxattr`"
                        );
                    }
                    if value.is_some() {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "the `userxattr` mount option does not take a value"
                        );
                    }
                    is_userxattr = true;
                }
                "volatile" => {
                    // This bare option aliases `fsync=volatile` (Linux
                    // `fs/overlayfs/params.c:690-692`) and shares its
                    // duplicate slot.
                    if fsync_mode.is_some() {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "duplicate overlay mount option `volatile`"
                        );
                    }
                    if value.is_some() {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "the `volatile` mount option does not take a value"
                        );
                    }
                    fsync_mode = Some(FsyncMode::Volatile);
                }
                "nooverride_creds" => {
                    // Caller-credential checks are the only local permission
                    // model, so the negated form is accepted as a silent
                    // no-op.
                    if is_nooverride_creds_seen {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "duplicate overlay mount option `nooverride_creds`"
                        );
                    }
                    if value.is_some() {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "the `nooverride_creds` mount option does not take a value"
                        );
                    }
                    is_nooverride_creds_seen = true;
                }
                "override_creds" => {
                    // The VFS provides no credentials-stash/override surface,
                    // so the requested semantic cannot be provided; upstream
                    // answers the same gap with `EINVAL`
                    // (`fs/overlayfs/params.c:705-708`). Rejected fail-fast
                    // instead of a silent credential-semantic inversion;
                    // flip to implemented once the VFS provides the API.
                    return_errno_with_message!(
                        Errno::EINVAL,
                        "the `override_creds` mount option requires a VFS credentials API"
                    );
                }
                "datadir+" => {
                    // Data-only layers have no local concept; the only
                    // available degradation (a regular lower layer) would
                    // silently publish the data directory's entries in the
                    // merged view, so the option is rejected outright.
                    return_errno_with_message!(
                        Errno::EINVAL,
                        "the `datadir+` mount option is not supported (data-only layers are not implemented)"
                    );
                }
                _ => {
                    return_errno_with_message!(Errno::EINVAL, "unknown overlay mount option");
                }
            }
        }

        if lower_dirs.is_empty() {
            return_errno_with_message!(
                Errno::EINVAL,
                "the `lowerdir` mount option must be specified"
            );
        }
        if upper_dir.is_some() != work_dir.is_some() {
            return_errno_with_message!(
                Errno::EINVAL,
                "the `workdir` mount option is required if and only if `upperdir` is specified"
            );
        }

        let options = Self {
            lower_dirs,
            upper_dir,
            work_dir,
            is_forced_read_only: fs_flags.contains(FsFlags::RDONLY),
            is_default_permissions,
            is_userxattr,
            uuid_mode,
            xino_mode,
            redirect_dir,
            index,
            nfs_export,
            metacopy,
            verity,
            fsync_mode,
        };
        options.verify()?;
        Ok(options)
    }

    /// Phase 2 of parse (upstream parity: `ovl_fs_params_verify`, Linux
    /// `fs/overlayfs/params.c:876-1039`). Resolves cross-key conflicts over
    /// raw intent, then discloses one-shot degrades for explicitly requested
    /// features that are not implemented.
    ///
    /// A `self`-method because every input it needs is an own field; the
    /// parse layer stays fs-free, so every verdict here is computable from
    /// the option string alone. Conflict verdicts always precede degrade
    /// disclosures, and an unmentioned key (`None`) never conflicts and
    /// never logs.
    fn verify(&self) -> Result<()> {
        // Cross-key conflicts (upstream `pr_err` + `EINVAL` parity). Only
        // explicitly set values conflict: `userxattr` alone and `metacopy=on`
        // with `redirect_dir=on` are compatible.
        if self.is_userxattr {
            match self.redirect_dir {
                Some(RedirectDirMode::Off) => {
                    return_errno_with_message!(
                        Errno::EINVAL,
                        "conflicting overlay mount options: `userxattr` and `redirect_dir=off`"
                    );
                }
                Some(RedirectDirMode::Follow) => {
                    return_errno_with_message!(
                        Errno::EINVAL,
                        "conflicting overlay mount options: `userxattr` and `redirect_dir=follow`"
                    );
                }
                Some(RedirectDirMode::On) => {
                    return_errno_with_message!(
                        Errno::EINVAL,
                        "conflicting overlay mount options: `userxattr` and `redirect_dir=on`"
                    );
                }
                Some(RedirectDirMode::NoFollow) | None => {}
            }
            if self.metacopy == Some(true) {
                return_errno_with_message!(
                    Errno::EINVAL,
                    "conflicting overlay mount options: `userxattr` and `metacopy=on`"
                );
            }
        }
        if self.metacopy == Some(true) {
            match self.redirect_dir {
                Some(RedirectDirMode::Off) => {
                    return_errno_with_message!(
                        Errno::EINVAL,
                        "conflicting overlay mount options: `metacopy=on` and `redirect_dir=off`"
                    );
                }
                Some(RedirectDirMode::Follow) => {
                    return_errno_with_message!(
                        Errno::EINVAL,
                        "conflicting overlay mount options: `metacopy=on` and `redirect_dir=follow`"
                    );
                }
                Some(RedirectDirMode::NoFollow) => {
                    return_errno_with_message!(
                        Errno::EINVAL,
                        "conflicting overlay mount options: `metacopy=on` and `redirect_dir=nofollow`"
                    );
                }
                Some(RedirectDirMode::On) | None => {}
            }
        }
        if self.nfs_export == Some(true) && self.index == Some(false) {
            return_errno_with_message!(
                Errno::EINVAL,
                "conflicting overlay mount options: `nfs_export=on` and `index=off`"
            );
        }
        if self.nfs_export == Some(true) && self.metacopy == Some(true) {
            return_errno_with_message!(
                Errno::EINVAL,
                "conflicting overlay mount options: `nfs_export=on` and `metacopy=on`"
            );
        }

        // One-shot degrade disclosure (upstream implements these features, so
        // there is no upstream `pr_warn` parity): one `warn!` per explicitly
        // requested intent, then the mount proceeds with the pre-existing
        // local behavior. Honored values (`nofollow`, `off`, `auto`) stay
        // silent.
        match self.redirect_dir {
            Some(RedirectDirMode::Off) => {
                warn!(
                    "`redirect_dir=off`: degrading to redirect_dir=nofollow; directory redirects are neither recorded nor followed"
                );
            }
            Some(RedirectDirMode::Follow) => {
                warn!(
                    "`redirect_dir=follow`: degrading to redirect_dir=nofollow; directory redirects are neither recorded nor followed"
                );
            }
            Some(RedirectDirMode::On) => {
                warn!(
                    "`redirect_dir=on`: degrading to redirect_dir=nofollow; directory redirects are neither recorded nor followed"
                );
            }
            Some(RedirectDirMode::NoFollow) | None => {}
        }
        if self.index == Some(true) {
            warn!("`index=on`: degrading to index=off; no inode index is maintained");
        }
        if self.nfs_export == Some(true) {
            warn!(
                "`nfs_export=on`: degrading to nfs_export=off; no export file handles are encoded"
            );
        }
        if self.metacopy == Some(true) {
            warn!("`metacopy=on`: degrading to metacopy=off; copy-up always copies data");
        }
        match self.verity {
            Some(VerityMode::On) => {
                warn!("`verity=on`: degrading to verity=off; fs-verity digests are not enforced");
            }
            Some(VerityMode::Require) => {
                warn!(
                    "`verity=require`: degrading to verity=off; fs-verity digests are not enforced"
                );
            }
            Some(VerityMode::Off) | None => {}
        }
        match self.fsync_mode {
            Some(FsyncMode::Strict) => {
                warn!("`fsync=strict`: metadata/directory copy-up is not explicitly synced");
            }
            Some(FsyncMode::Volatile) => {
                warn!(
                    "`fsync=volatile`: sync suppression, the volatile dirty marker, and sticky syncfs errors are not implemented; durability follows the underlying filesystem"
                );
            }
            Some(FsyncMode::Auto) | None => {}
        }
        Ok(())
    }

    fn require_non_empty_value(value: &str, message: &'static str) -> Result<()> {
        if value.is_empty() {
            return_errno_with_message!(Errno::EINVAL, message);
        }
        Ok(())
    }
}

#[cfg(ktest)]
mod test {
    // SPDX-License-Identifier: MPL-2.0

    //! Unit tests for the pure [`MountOptions::parse`] contract (U-1).
    //!
    //! Every expectation below is the frozen U-1 case table of the test-assets
    //! design (`test-assets-20260831` §3.1) as amended by the MO6 delta
    //! (`mount-options-v2-20260831` §5). The tests assert the parse surface only:
    //! no filesystem, VFS, block, or I/O fixture is constructed.

    use ostd::prelude::ktest;

    use super::*;

    /// Parses `args` expecting success and returns the validated options.
    fn parse_expect_ok(args: &str, fs_flags: FsFlags) -> MountOptions {
        MountOptions::parse(Some(args), fs_flags).unwrap()
    }

    /// Parses `args` expecting `EINVAL` and returns the error for errno checks.
    fn parse_expect_einval(args: Option<&str>, fs_flags: FsFlags) -> Error {
        let err = MountOptions::parse(args, fs_flags).unwrap_err();
        assert_eq!(err.error(), Errno::EINVAL);
        err
    }

    #[ktest]
    fn parse_requires_lowerdir() {
        // `none` is treated as a missing `lowerdir`.
        parse_expect_einval(None, FsFlags::empty());
        // `empty`: all entries empty -> no `lowerdir`.
        parse_expect_einval(Some(""), FsFlags::empty());
        // `separators only`: empty entries skipped, then missing `lowerdir`.
        parse_expect_einval(Some(",,"), FsFlags::empty());
        // `empty value`.
        parse_expect_einval(Some("lowerdir="), FsFlags::empty());
        // `empty layer`.
        parse_expect_einval(Some("lowerdir=a::b"), FsFlags::empty());
        parse_expect_einval(Some("lowerdir=:"), FsFlags::empty());
        parse_expect_einval(Some("lowerdir=a:"), FsFlags::empty());
    }

    #[ktest]
    fn parse_lowerdir_layer_list() {
        // `single layer`.
        let options = parse_expect_ok("lowerdir=a", FsFlags::empty());
        assert_eq!(options.lower_dirs, ["a"]);
        // `layer order`: the first path is the topmost layer.
        let options = parse_expect_ok("lowerdir=a:b:c", FsFlags::empty());
        assert_eq!(options.lower_dirs, ["a", "b", "c"]);
        // `rdonly flag`.
        let read_only = parse_expect_ok("lowerdir=l", FsFlags::RDONLY);
        assert!(read_only.is_forced_read_only);
        let writable = parse_expect_ok("lowerdir=l", FsFlags::empty());
        assert!(!writable.is_forced_read_only);
    }

    #[ktest]
    fn parse_upperdir_workdir_pairing() {
        // Upper without work / work without upper.
        parse_expect_einval(Some("lowerdir=l,upperdir=u"), FsFlags::empty());
        parse_expect_einval(Some("lowerdir=l,workdir=w"), FsFlags::empty());
        // Both present.
        let options = parse_expect_ok("lowerdir=l,upperdir=u,workdir=w", FsFlags::empty());
        assert_eq!(options.upper_dir.as_deref(), Some("u"));
        assert_eq!(options.work_dir.as_deref(), Some("w"));
    }

    #[ktest]
    fn parse_rejects_duplicate_keys() {
        // Duplicate key (each of the 7): the first occurrence never silently wins.
        parse_expect_einval(Some("lowerdir=a,lowerdir=b"), FsFlags::empty());
        parse_expect_einval(Some("lowerdir=l,upperdir=u,upperdir=v"), FsFlags::empty());
        parse_expect_einval(Some("lowerdir=l,workdir=w,workdir=v"), FsFlags::empty());
        parse_expect_einval(Some("lowerdir=l,uuid=on,uuid=off"), FsFlags::empty());
        parse_expect_einval(Some("lowerdir=l,xino=on,xino=off"), FsFlags::empty());
        parse_expect_einval(
            Some("lowerdir=l,default_permissions,default_permissions"),
            FsFlags::empty(),
        );
        parse_expect_einval(Some("lowerdir=l,userxattr,userxattr"), FsFlags::empty());
    }

    #[ktest]
    fn parse_rejects_bare_valued_keys() {
        // Bare valued key (bare, no `=`).
        parse_expect_einval(Some("lowerdir"), FsFlags::empty());
        parse_expect_einval(Some("lowerdir=l,upperdir"), FsFlags::empty());
        parse_expect_einval(Some(",workdir"), FsFlags::empty());
        parse_expect_einval(Some(",uuid"), FsFlags::empty());
        parse_expect_einval(Some(",xino"), FsFlags::empty());
        // Unknown key / empty key / case-sensitive key.
        parse_expect_einval(Some("foo=bar"), FsFlags::empty());
        parse_expect_einval(Some("foo"), FsFlags::empty());
        parse_expect_einval(Some("=value"), FsFlags::empty());
        parse_expect_einval(Some("LOWERDIR=l"), FsFlags::empty());
    }

    #[ktest]
    fn parse_valueless_keys() {
        // Valueless keys, bare.
        let options = parse_expect_ok("lowerdir=l,default_permissions", FsFlags::empty());
        assert!(options.is_default_permissions);
        let options = parse_expect_ok("lowerdir=l,userxattr", FsFlags::empty());
        assert!(options.is_userxattr);
        let options = parse_expect_ok("lowerdir=l,default_permissions,userxattr", FsFlags::empty());
        assert!(options.is_default_permissions);
        assert!(options.is_userxattr);
        // Valueless keys, with value (an empty string is still a value).
        parse_expect_einval(Some("default_permissions=x"), FsFlags::empty());
        parse_expect_einval(Some("userxattr=1"), FsFlags::empty());
        parse_expect_einval(Some("userxattr="), FsFlags::empty());
    }

    #[ktest]
    fn parse_uuid_and_xino_values() {
        // `uuid` domain.
        assert_eq!(
            parse_expect_ok("lowerdir=l,uuid=off", FsFlags::empty()).uuid_mode,
            Some(UuidMode::Off)
        );
        assert_eq!(
            parse_expect_ok("lowerdir=l,uuid=null", FsFlags::empty()).uuid_mode,
            Some(UuidMode::Null)
        );
        assert_eq!(
            parse_expect_ok("lowerdir=l,uuid=on", FsFlags::empty()).uuid_mode,
            Some(UuidMode::On)
        );
        assert_eq!(
            parse_expect_ok("lowerdir=l,uuid=auto", FsFlags::empty()).uuid_mode,
            Some(UuidMode::Auto)
        );
        // `uuid` invalid.
        parse_expect_einval(Some("uuid="), FsFlags::empty());
        parse_expect_einval(Some("uuid=ON"), FsFlags::empty());
        parse_expect_einval(Some("uuid=yes"), FsFlags::empty());
        // `xino` domain.
        assert_eq!(
            parse_expect_ok("lowerdir=l,xino=off", FsFlags::empty()).xino_mode,
            Some(XinoMode::Off)
        );
        assert_eq!(
            parse_expect_ok("lowerdir=l,xino=auto", FsFlags::empty()).xino_mode,
            Some(XinoMode::Auto)
        );
        assert_eq!(
            parse_expect_ok("lowerdir=l,xino=on", FsFlags::empty()).xino_mode,
            Some(XinoMode::On)
        );
        // `xino` invalid.
        parse_expect_einval(Some("xino="), FsFlags::empty());
        parse_expect_einval(Some("xino=ON"), FsFlags::empty());
        parse_expect_einval(Some("xino=1"), FsFlags::empty());
        parse_expect_einval(Some("xino=on "), FsFlags::empty());
    }

    #[ktest]
    fn parse_literal_value_semantics() {
        // A value containing `=`: the first `=` splits, the rest is a literal
        // value.
        let options = parse_expect_ok("lowerdir=a=b", FsFlags::empty());
        assert_eq!(options.lower_dirs, ["a=b"]);
        // A quoted value is literal: no unquoting.
        let options = parse_expect_ok("lowerdir=\"a\"", FsFlags::empty());
        assert_eq!(options.lower_dirs, ["\"a\""]);
        // Whitespace is literal.
        let options = parse_expect_ok("lowerdir=a b", FsFlags::empty());
        assert_eq!(options.lower_dirs, ["a b"]);
        let options = parse_expect_ok("lowerdir= a", FsFlags::empty());
        assert_eq!(options.lower_dirs, [" a"]);
        parse_expect_einval(Some("lowerdir=l,uuid=on "), FsFlags::empty());
        // A comma cannot be escaped: the entry `b` is an unknown key.
        parse_expect_einval(Some("lowerdir=a,b"), FsFlags::empty());
        // Empty entries are skipped.
        let options = parse_expect_ok(
            ",,lowerdir=l,,userxattr,,default_permissions,,",
            FsFlags::empty(),
        );
        assert!(options.is_userxattr);
        assert!(options.is_default_permissions);
    }

    #[ktest]
    fn parse_redirect_dir_domain() {
        // `redirect_dir` domain.
        assert_eq!(
            parse_expect_ok("lowerdir=l,redirect_dir=on", FsFlags::empty()).redirect_dir,
            Some(RedirectDirMode::On)
        );
        assert_eq!(
            parse_expect_ok("lowerdir=l,redirect_dir=follow", FsFlags::empty()).redirect_dir,
            Some(RedirectDirMode::Follow)
        );
        assert_eq!(
            parse_expect_ok("lowerdir=l,redirect_dir=nofollow", FsFlags::empty()).redirect_dir,
            Some(RedirectDirMode::NoFollow)
        );
        assert_eq!(
            parse_expect_ok("lowerdir=l,redirect_dir=off", FsFlags::empty()).redirect_dir,
            Some(RedirectDirMode::Off)
        );
        // `redirect_dir` invalid.
        parse_expect_einval(Some("redirect_dir="), FsFlags::empty());
        parse_expect_einval(Some("redirect_dir=ON"), FsFlags::empty());
        parse_expect_einval(Some("redirect_dir=yes"), FsFlags::empty());
        parse_expect_einval(Some("redirect_dir"), FsFlags::empty());
        parse_expect_einval(Some("redirect_dir=on,redirect_dir=off"), FsFlags::empty());
    }

    #[ktest]
    fn parse_bool_option_domains() {
        // Bool domain (`index`/`nfs_export`/`metacopy`).
        assert_eq!(
            parse_expect_ok("lowerdir=l,index=on", FsFlags::empty()).index,
            Some(true)
        );
        assert_eq!(
            parse_expect_ok("lowerdir=l,index=off", FsFlags::empty()).index,
            Some(false)
        );
        assert_eq!(
            parse_expect_ok("lowerdir=l,nfs_export=on", FsFlags::empty()).nfs_export,
            Some(true)
        );
        assert_eq!(
            parse_expect_ok("lowerdir=l,nfs_export=off", FsFlags::empty()).nfs_export,
            Some(false)
        );
        assert_eq!(
            parse_expect_ok("lowerdir=l,metacopy=on", FsFlags::empty()).metacopy,
            Some(true)
        );
        assert_eq!(
            parse_expect_ok("lowerdir=l,metacopy=off", FsFlags::empty()).metacopy,
            Some(false)
        );
        // Bool invalid (each).
        parse_expect_einval(Some("index=1"), FsFlags::empty());
        parse_expect_einval(Some("index=ON"), FsFlags::empty());
        parse_expect_einval(Some("index="), FsFlags::empty());
        parse_expect_einval(Some("index"), FsFlags::empty());
        parse_expect_einval(Some("index=on,index=off"), FsFlags::empty());
    }

    #[ktest]
    fn parse_verity_and_fsync_domains() {
        // `verity` domain.
        assert_eq!(
            parse_expect_ok("lowerdir=l,verity=off", FsFlags::empty()).verity,
            Some(VerityMode::Off)
        );
        assert_eq!(
            parse_expect_ok("lowerdir=l,verity=on", FsFlags::empty()).verity,
            Some(VerityMode::On)
        );
        assert_eq!(
            parse_expect_ok("lowerdir=l,verity=require", FsFlags::empty()).verity,
            Some(VerityMode::Require)
        );
        // `verity` invalid.
        parse_expect_einval(Some("verity=required"), FsFlags::empty());
        parse_expect_einval(Some("verity="), FsFlags::empty());
        parse_expect_einval(Some("verity"), FsFlags::empty());
        parse_expect_einval(Some("verity=on,verity=off"), FsFlags::empty());
        // `fsync` domain.
        assert_eq!(
            parse_expect_ok("lowerdir=l,fsync=auto", FsFlags::empty()).fsync_mode,
            Some(FsyncMode::Auto)
        );
        assert_eq!(
            parse_expect_ok("lowerdir=l,fsync=strict", FsFlags::empty()).fsync_mode,
            Some(FsyncMode::Strict)
        );
        assert_eq!(
            parse_expect_ok("lowerdir=l,fsync=volatile", FsFlags::empty()).fsync_mode,
            Some(FsyncMode::Volatile)
        );
        // `fsync` invalid.
        parse_expect_einval(Some("fsync=0"), FsFlags::empty());
        parse_expect_einval(Some("fsync="), FsFlags::empty());
        parse_expect_einval(Some("fsync"), FsFlags::empty());
        parse_expect_einval(Some("fsync=auto,fsync=strict"), FsFlags::empty());
    }

    #[ktest]
    fn parse_volatile_alias() {
        // The valueless `volatile` option aliases `fsync=volatile`.
        assert_eq!(
            parse_expect_ok("lowerdir=l,volatile", FsFlags::empty()).fsync_mode,
            Some(FsyncMode::Volatile)
        );
        // The alias shares the `fsync` duplicate slot.
        parse_expect_einval(Some("volatile=1"), FsFlags::empty());
        parse_expect_einval(Some("volatile="), FsFlags::empty());
        parse_expect_einval(Some("volatile,volatile"), FsFlags::empty());
        parse_expect_einval(Some("volatile,fsync=auto"), FsFlags::empty());
        parse_expect_einval(Some("fsync=auto,volatile"), FsFlags::empty());
    }

    #[ktest]
    fn parse_override_creds_forms() {
        // Positive `override_creds` is rejected.
        parse_expect_einval(Some("lowerdir=l,override_creds"), FsFlags::empty());
        parse_expect_einval(Some("override_creds=on"), FsFlags::empty());
        // `nooverride_creds` is accepted; no field is set.
        let options = parse_expect_ok("lowerdir=l,nooverride_creds", FsFlags::empty());
        assert_eq!(options.redirect_dir, None);
        assert_eq!(options.index, None);
        assert_eq!(options.nfs_export, None);
        assert_eq!(options.metacopy, None);
        assert_eq!(options.verity, None);
        assert_eq!(options.fsync_mode, None);
        // `nooverride_creds` rejected forms.
        parse_expect_einval(Some("nooverride_creds=x"), FsFlags::empty());
        parse_expect_einval(Some("nonooverride_creds"), FsFlags::empty());
        parse_expect_einval(Some("nooverride_creds,nooverride_creds"), FsFlags::empty());
    }

    #[ktest]
    fn parse_lowerdir_plus_append() {
        // Each `lowerdir+` occurrence appends one path.
        let options = parse_expect_ok("lowerdir+=/a,lowerdir+=/b", FsFlags::empty());
        assert_eq!(options.lower_dirs, ["/a", "/b"]);
        // `lowerdir+` takes one verbatim path: colons are not split.
        let options = parse_expect_ok("lowerdir+=/a:b", FsFlags::empty());
        assert_eq!(options.lower_dirs, ["/a:b"]);
        // `lowerdir+` cannot be combined with `lowerdir`.
        parse_expect_einval(Some("lowerdir=/l,lowerdir+=/a"), FsFlags::empty());
        parse_expect_einval(Some("lowerdir+=/a,lowerdir=/l"), FsFlags::empty());
        parse_expect_einval(Some("lowerdir+="), FsFlags::empty());
        // The comma limitation extends to `lowerdir+`: the entry `b` is an
        // unknown key.
        parse_expect_einval(Some("lowerdir+=/a,b"), FsFlags::empty());
    }

    #[ktest]
    fn parse_datadir_plus_rejected() {
        parse_expect_einval(Some("lowerdir=l,datadir+=/d"), FsFlags::empty());
    }

    #[ktest]
    fn parse_new_key_conflicts() {
        // `userxattr` x `redirect_dir` (anything but `nofollow`).
        parse_expect_einval(Some("lowerdir=l,userxattr,redirect_dir=on"), FsFlags::empty());
        parse_expect_einval(Some("lowerdir=l,userxattr,redirect_dir=follow"), FsFlags::empty());
        parse_expect_einval(Some("lowerdir=l,userxattr,redirect_dir=off"), FsFlags::empty());
        // `userxattr` x `redirect_dir=nofollow` is compatible.
        parse_expect_ok("lowerdir=l,userxattr,redirect_dir=nofollow", FsFlags::empty());
        // `userxattr` x `metacopy`.
        parse_expect_einval(Some("lowerdir=l,userxattr,metacopy=on"), FsFlags::empty());
        parse_expect_ok("lowerdir=l,userxattr,metacopy=off", FsFlags::empty());
        // `metacopy=on` x `redirect_dir` (anything but `on`).
        parse_expect_einval(Some("lowerdir=l,metacopy=on,redirect_dir=nofollow"), FsFlags::empty());
        parse_expect_einval(Some("lowerdir=l,metacopy=on,redirect_dir=off"), FsFlags::empty());
        parse_expect_einval(Some("lowerdir=l,metacopy=on,redirect_dir=follow"), FsFlags::empty());
        // `metacopy=on` x `redirect_dir=on` is compatible.
        parse_expect_ok("lowerdir=l,metacopy=on,redirect_dir=on", FsFlags::empty());
        // `nfs_export=on` x `index=off`.
        parse_expect_einval(Some("lowerdir=l,nfs_export=on,index=off"), FsFlags::empty());
        // `nfs_export=on` x `metacopy=on`.
        parse_expect_einval(Some("lowerdir=l,nfs_export=on,metacopy=on"), FsFlags::empty());
    }
}
