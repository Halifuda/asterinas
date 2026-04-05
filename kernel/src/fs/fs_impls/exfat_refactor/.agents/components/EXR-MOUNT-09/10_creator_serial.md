<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Serial Handoff

## Metadata

- Component ID: `EXR-MOUNT-09`
- Title: Mount Bootstrap And Shared Filesystem State
- Status: `SerialImplementing`
- Author: `main-agent`
- Date: `2026-04-05`
- Task packet: locally executed by `main-agent` in the `ember-causeway` wave; no delegated creator packet
- Implemented spec:
  - `00_architect.md`
  - `01_designer_core.md`
  - `02_designer_async.md`
  - `03_designer_ktest.md`
- Pass kind: `serial implementation`

## Planned File Ownership

- Files to edit:
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/10_creator_serial.md`
- Files intentionally left untouched:
  - `kernel/src/fs/fs_impls/exfat_refactor/sysroot.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
  - all checker-owned test additions and checker artifacts

## Implementation Notes

Implemented the mount-owned bootstrap surface in the new `fs.rs` module and wired it into `mod.rs`.

The pass now provides:

- one canonical `ExfatFs::mount(...)` constructor that consumes validated superblock facts plus accepted root discovery facts,
- one mount-owned shared state object that stores the block device, validated superblock copy, loaded upcase table, loaded allocation bitmap, and synthetic root inode shell,
- one explicit root-seeding path that uses `ExfatInodeMeta::new_root(...)` instead of the ordinary inode constructor,
- one small `ExfatChain::byte_len(...)` helper so mount can derive the synthetic root size from validated chain facts without opening a second chain-length surface.

The implementation stays within the specified boundary:

- no root rescanning,
- no directory lookup policy,
- no page-cache backend behavior,
- no bitmap mutation or allocation policy,
- no async machinery beyond one-shot private construction and return-by-value publication.

## Approved Deviations

- `ExfatFs::mount(...)` takes `Arc<dyn BlockDevice>` rather than `&dyn BlockDevice` so the returned mount-owned filesystem object can retain the device handle as shared runtime state. This preserves the designer's ownership intent without adding a second publication step.

## Optional Self-Checks

- Commands run, if any:
  - None.
- Compile checks run, if any:
  - None.
- Manual reasoning checks:
  - The constructor returns the filesystem object only after both dependent loaders and root seeding succeed.
  - The accepted root discovery aggregate is consumed directly and not stored as a second long-lived helper layer.
  - The root shell remains explicit and synthetic.

## Remaining Risks

- Checker coverage still needs to prove the happy path, missing-fact rejection, and failure-atomicity cases in `fs.rs`.
- The new mount module has not yet been compile- or ktest-validated in the shared container.
- Later components may need targeted read-only accessors on `ExfatFs`, but this creator pass intentionally avoided widening that helper surface before a named downstream caller required it.
