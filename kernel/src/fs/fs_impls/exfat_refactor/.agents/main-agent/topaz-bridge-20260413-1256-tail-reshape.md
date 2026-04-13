<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff

## Metadata

- Fancy nickname: `topaz-bridge`
- Date: `2026-04-13 12:56 CST`
- Covered hours: post-`amber-delta` board reshape requested by user; architect subagent audit plus main-agent Linux/Asterinas VFS audit
- Author: `main-agent`
- Workspace: `/home/halifuda/asterinas`
- Container or environment: shared host workspace plus shared Docker container `codex-asterinas-dev`
- Status: post-28 board tail has been reshaped; `EXR-NAMESPACE-29` is now blocked on `EXR-CHARSET-32`; `EXR-WRITE-30` remains specified but buffered-only; new planned rows now include `EXR-CHARSET-32`, `EXR-DIRECT-33`, `EXR-BOOT-34`, `EXR-VOLLABEL-35`, and `EXR-INODE-META-36`

## Environment Summary

- Checker execution is still serialized through `.agents/tools/checker_lock.sh`.
- No checker lane was opened in this wave.
- This wave was artifact-only scheduling and board maintenance.

## Current Project State

- Accepted rows still include everything through `EXR-DENTRY-WRITE-28`.
- `EXR-NAMESPACE-29`:
  - state changed from `Specified` to `Blocked`
  - new dependency: `EXR-CHARSET-32`
- `EXR-WRITE-30`:
  - still `Specified`
  - now explicitly buffered-only; `O_DIRECT` is no longer an implicit subcase
- `EXR-SYNC-31`:
  - still `Planned`
  - explicitly narrowed to flush ordering only
- New planned rows:
  - `EXR-CHARSET-32`
  - `EXR-DIRECT-33`
  - `EXR-BOOT-34`
  - `EXR-VOLLABEL-35`
  - `EXR-INODE-META-36`
- Explicit non-goals for the current board unless reopened as separate rows:
  - FAT-attribute ioctls
  - trim/discard
  - forced shutdown

## Recent Decisions

- Archived and delegated an architect packet at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/WORKSPACE-ARCH-POST28/20260413-1248-architect-packet.md`.
- Accepted the architect proposal artifact at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-POST28/00_architect.md` as the board-reshape authority for omitted post-28 surfaces.
- Performed a main-agent local audit against:
  - `.agents/Microsoft-exFAT-spec.md`
  - `.agents/linux-exFAT-implementation-summary.md`
  - `.agents/ASTERINAS_ARCHITECT_PRIORS.md`
  - Linux `fs/exfat/{super,file,inode,namei,dir,nls}.c`
  - current refactor `inode.rs` / `fs.rs`
  - Asterinas VFS `kernel/src/fs/vfs/fs_apis/inode.rs`
- Updated `.agents/COMPONENT_INDEX.md`.
  - Added a `2026-04-13` post-tail audit note.
  - Recut `EXR-NAMESPACE-29` as blocked pending `EXR-CHARSET-32`.
  - Reaffirmed `EXR-WRITE-30` as buffered-only.
  - Reaffirmed `EXR-SYNC-31` as sync-only.
  - Added rows `32` through `36`.
- Added `EXR-INODE-META-36` beyond the architect subagent's proposal because the local VFS audit showed `set_mode`, `set_owner`, `set_group`, and explicit timestamp setters are still ownerless stubs in `inode.rs`; leaving them off-board would preserve a real gap even after `31`.

## Open Risks And Assumptions

- `EXR-NAMESPACE-29` should not resume creator work from the existing designer set; that design predates the new explicit charset/name-conversion boundary.
- `EXR-WRITE-30` is still the sharpest production functionality gap because `write_at` and `resize` remain stubbed.
- `EXR-DIRECT-33`, `EXR-NAMESPACE-29`, `EXR-WRITE-30`, and `EXR-INODE-META-36` will all collide in `inode.rs`; do not treat them as file-parallel creator lanes.
- `EXR-CHARSET-32`, `EXR-BOOT-34`, `EXR-VOLLABEL-35`, and `EXR-SYNC-31` will all collide in `fs.rs`; keep those creator waves serialized too.
- `EXR-CHARSET-32` still needs a design decision about whether Asterinas closes Linux NLS parity or records UTF-8-only external naming as an explicit non-goal inside the row.

## Recommended Next Actions

1. Architect and design `EXR-CHARSET-32` first so `EXR-NAMESPACE-29` can be repaired against a stable codec boundary.
2. Keep `EXR-WRITE-30` as the next creator frontier in parallel planning, but do not reopen its scope to include `O_DIRECT` or inode-admin metadata control.
3. After `EXR-CHARSET-32`, issue an architect/designer repair for `EXR-NAMESPACE-29`.
4. Leave `EXR-SYNC-31` narrow when it is eventually architected; do not use it as a bucket for volume label, boot policy, or admin-control cleanup.

## Resume Checklist

- Use `$exfat-main-agent`.
- Read `README.md`.
- Read `COMPONENT_INDEX.md`.
- Read this handoff after `amber-delta`.
- Read `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-POST28/00_architect.md`.
- Treat `EXR-NAMESPACE-29` as blocked on `EXR-CHARSET-32`.
- Treat `EXR-WRITE-30` as specified but buffered-only.
- Treat `EXR-DIRECT-33`, `EXR-BOOT-34`, `EXR-VOLLABEL-35`, and `EXR-INODE-META-36` as real planned rows, not handoff footnotes.
