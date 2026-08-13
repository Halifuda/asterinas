// SPDX-License-Identifier: MPL-2.0

//! Mount option parsing for overlayfs.
//!
//! This module validates the mount option string into an
//! [`OverlayMountOptions`] construction input. The recognized keys are the
//! [`MountOptionKey`] variants; unknown keys fail with `EINVAL` before any
//! layer state is created.
//!
//! ## References
//!
//! - <https://elixir.bootlin.com/linux/v7.0/source/Documentation/filesystems/overlayfs.rst#L350-L364>
//!   (Linux stacks colon-separated lowerdirs with the first entry topmost)

use crate::{fs::vfs::file_system::FsFlags, prelude::*};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MountOptionKey {
    LowerDir,
    UpperDir,
    WorkDir,
    Uuid,
    DefaultPermissions,
    Xino,
}

/// The UUID/`fsid` policy of an overlay mount.
///
/// The default is [`UuidMode::Auto`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum UuidMode {
    /// The overlay UUID is null and the fsid comes from the topmost underlying fs.
    Off,
    /// Same as [`UuidMode::Off`], plus underlying-layer UUIDs are ignored.
    Null,
    /// The overlay UUID is generated and persisted as `trusted.overlay.uuid`.
    On,
    /// Reuse an existing persisted UUID or upgrade to `On`; degrade to `Null`.
    Auto,
}

/// The `xino` mode.
///
/// The default is [`XinoMode::Auto`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in overlayfs) enum XinoMode {
    /// xino encoding disabled; non-directories report the underlying dev/ino.
    Off,
    /// xino enabled when feasible (the default).
    Auto,
    /// xino encoding always enabled.
    On,
}

/// Validated construction input for an overlay mount.
///
/// The struct is constructed once by [`OverlayMountOptions::parse`],
/// consumed once by `OverlayFs::new`.
#[derive(Debug)]
pub(super) struct OverlayMountOptions {
    /// Lower layer paths in option order; the first option is the topmost.
    pub(super) lower_dirs: Vec<String>,
    /// Upper layer path; `None` means a read-only overlay.
    pub(super) upper_dir: Option<String>,
    /// Work directory path; `Some` iff `upper_dir` is `Some`.
    pub(super) work_dir: Option<String>,
    pub(super) is_forced_read_only: bool,
    pub(super) is_default_permissions: bool,
    /// The UUID persistence mode; defaults to [`UuidMode::Auto`].
    pub(super) uuid_mode: UuidMode,
    /// The `xino=` mode; defaults to [`XinoMode::Auto`].
    pub(super) xino_mode: XinoMode,
}

impl OverlayMountOptions {
    /// Parses the comma-separated mount option string plus the mount flags
    /// into a validated [`OverlayMountOptions`].
    ///
    /// Parsing contract (all violations fail with `EINVAL`):
    ///
    /// * Key/value and splitting: the string is split on `,`, empty entries are
    ///   skipped, and `key=value` sets the key's value; a bare `key` is a
    ///   valueless option accepted only for `default_permissions`.
    /// * Key domains and repetition: `lowerdir` is required and is a
    ///   non-empty, colon-separated layer list (first path topmost) with no
    ///   empty layer; `upperdir`/`workdir` each take a single non-empty path
    ///   and must both be present or both be absent; `uuid` accepts only
    ///   `off`/`null`/`on`/`auto`, `xino` accepts only `off`/`auto`/`on`,
    ///   and `default_permissions` takes no value; every key may appear at
    ///   most once.
    /// * Required-value constraint: `None` (no option string) fails like a
    ///   missing `lowerdir`.
    pub(super) fn parse(args: Option<&str>, fs_flags: FsFlags) -> Result<Self> {
        let mut lower_dirs = Vec::new();
        let mut upper_dir = None;
        let mut work_dir = None;
        let mut is_default_permissions = false;
        let mut uuid_mode = UuidMode::Auto;
        // Boolean markers record which keys were seen, because some options
        // have default values.
        let mut saw_uuid = false;
        let mut xino_mode = XinoMode::Auto;
        let mut saw_xino = false;

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
            let key = match key_name {
                "lowerdir" => MountOptionKey::LowerDir,
                "upperdir" => MountOptionKey::UpperDir,
                "workdir" => MountOptionKey::WorkDir,
                "uuid" => MountOptionKey::Uuid,
                "xino" => MountOptionKey::Xino,
                "default_permissions" => MountOptionKey::DefaultPermissions,
                _ => {
                    return_errno_with_message!(Errno::EINVAL, "unknown overlay mount option");
                }
            };
            match key {
                MountOptionKey::LowerDir => {
                    if !lower_dirs.is_empty() {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "duplicate overlay mount option `lowerdir`"
                        );
                    }
                    let Some(value) = value else {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "the `lowerdir` mount option requires a value"
                        );
                    };
                    if value.is_empty() {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "the `lowerdir` mount option requires a non-empty value"
                        );
                    }
                    lower_dirs = value.split(':').map(str::to_string).collect();
                    if lower_dirs.iter().any(|lower_dir| lower_dir.is_empty()) {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "the `lowerdir` value contains an empty layer path"
                        );
                    }
                }
                MountOptionKey::UpperDir => {
                    if upper_dir.is_some() {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "duplicate overlay mount option `upperdir`"
                        );
                    }
                    let Some(value) = value else {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "the `upperdir` mount option requires a value"
                        );
                    };
                    if value.is_empty() {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "the `upperdir` mount option requires a non-empty value"
                        );
                    }
                    upper_dir = Some(value.to_string());
                }
                MountOptionKey::WorkDir => {
                    if work_dir.is_some() {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "duplicate overlay mount option `workdir`"
                        );
                    }
                    let Some(value) = value else {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "the `workdir` mount option requires a value"
                        );
                    };
                    if value.is_empty() {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "the `workdir` mount option requires a non-empty value"
                        );
                    }
                    work_dir = Some(value.to_string());
                }
                MountOptionKey::Uuid => {
                    if saw_uuid {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "duplicate overlay mount option `uuid`"
                        );
                    }
                    saw_uuid = true;
                    let Some(value) = value else {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "the `uuid` mount option requires a value"
                        );
                    };
                    uuid_mode = match value {
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
                    };
                }
                MountOptionKey::Xino => {
                    if saw_xino {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "duplicate overlay mount option `xino`"
                        );
                    }
                    saw_xino = true;
                    let Some(value) = value else {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "the `xino` mount option requires a value"
                        );
                    };
                    xino_mode = match value {
                        "off" => XinoMode::Off,
                        "auto" => XinoMode::Auto,
                        "on" => XinoMode::On,
                        _ => {
                            return_errno_with_message!(
                                Errno::EINVAL,
                                "invalid `xino` mount option value"
                            );
                        }
                    };
                }
                MountOptionKey::DefaultPermissions => {
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

        Ok(Self {
            lower_dirs,
            upper_dir,
            work_dir,
            is_forced_read_only: fs_flags.contains(FsFlags::RDONLY),
            is_default_permissions,
            uuid_mode,
            xino_mode,
        })
    }
}
