<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Serial Pass

## Metadata

- Component ID: `EXR-BOOT-34`
- Role: `creator`
- Pass: `serial creator`
- File scope:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## What Changed

- Added owner-private boot-policy carriers in `fs.rs`:
  - `BootSource`
  - `BootDirtyIntent`
  - `BootPolicySnapshot`
  - `BootPolicyState`
- Added an owner-private publication helper on `ExfatFs`:
  - `publish_boot_policy(fallback_candidate, percent_in_use)`
- Added an owner-private selector helper on `ExfatFs`:
  - `select_trusted_boot_source(...)`
- Added an owner-private dirty-intent projection helper on `ExfatFs`:
  - `published_boot_dirty_intent()`
- Updated `ExfatFs::open_root_inode()` to publish the boot-policy snapshot before it exposes the ready root inode.

## Final Ownership

- `BootSource` remains owner-private to `ExfatFs`.
- `BootPolicySnapshot` remains owner-private to `ExfatFs`.
- `BootDirtyIntent` remains owner-private to `ExfatFs`.
- `BootPolicyState` remains owner-private to `ExfatFs`.
- `publish_boot_policy()` remains owner-private to `ExfatFs`.
- `published_boot_dirty_intent()` remains owner-private to `ExfatFs` for later sync consumption.

## Policy Notes

- The mount/open path stays primary-default when no fallback candidate is provided.
- `percent_in_use` is carried as an optional observation slot and is published as `None` in the current production path.
- The dirty-intent projection keeps the sync-facing view separate from the boot-source decision.
- `ClearToZero` is currently represented by the second persistent boot-region flag bit in the snapshot carrier, so later mutation code can consume it without reopening parsing ownership.

## Stop Condition

- Serial creator work is complete.
- No checker work was started.
- No compile, test, format, Docker, or QEMU command was run.
