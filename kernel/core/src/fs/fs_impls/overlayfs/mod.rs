// SPDX-License-Identifier: MPL-2.0

// The legacy single-file implementation is frozen and lives in `legacy_fs.rs`.
// It remains the ACTIVE registered overlay filesystem until the refactor
// explicitly schedules a takeover. The ONLY permitted reference to it (for the
// refactor) is this registration wiring (`OverlayFsType` + `register()`); it is
// NOT a design source for any Creator pass.
use legacy_fs::OverlayFsType;

mod legacy_fs;

mod mount;

pub(super) fn init() {
    crate::fs::vfs::registry::register(&OverlayFsType).unwrap();
}
