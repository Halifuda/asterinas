<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-SYNC-31-20260413-1304-DESIGN`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-SYNC-31/20260413-1304-designer-packet.md`
- Supersedes: None
- Role: `designer`
- Component: `EXR-SYNC-31`
- Phase: `designer`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 13:04 CST`

## Goal

- Produce the split designer artifact set for `EXR-SYNC-31` so later creator work can implement `ExfatFs`-owned sync, inode sync delegation, and page-cache writeback ordering without guessing about owner boundaries, dirty-producer handoffs, or checker obligations.

## Architectural Unit Context

- Functional goal: `ExfatFs` sync and flush-ordering owner boundary
- Final architectural owner: `ExfatFs`
- Expected landing form: owner methods plus owner-private dirty-state helpers
- Architect artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYNC-31/00_architect.md`

## Required Resolution Questions

- Specify the smallest `ExfatFs`-owned sync surface that covers:
  - `FileSystem::sync()`
  - `Inode::sync_all()`
  - `Inode::sync_data()`
  - `PageCacheBackend::write_page_async()`
  while staying a flush-ordering boundary only.
- State exactly how the row consumes dirty producers from `EXR-WRITE-30` and `EXR-NAMESPACE-29`, and how later producers such as `EXR-VOLLABEL-35`, `EXR-INODE-META-36`, and possible boot-flag dirty output from `EXR-BOOT-34` remain downstream consumers rather than reasons to widen the row.
- Keep control-path policy out of scope: no direct I/O, no name conversion, no boot fallback decisions, no volume-label user control, no FAT-attribute ioctls, no trim/discard, and no forced shutdown.
- Define narrow creator and checker obligations so later work does not guess where inode-local dirty production ends and filesystem-wide persistence ordering begins.
- State serialization, delegation, and repeated-call expectations for `sync()`, `sync_all()`, `sync_data()`, and `write_page_async()` without inventing a public writeback manager.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYNC-31/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/30_reviewer_report.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-POST28/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/file_system.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/linux/fs/exfat/exfat_fs.h`
- `/home/halifuda/linux/fs/exfat/file.c`
- `/home/halifuda/linux/fs/exfat/inode.c`
- `/home/halifuda/linux/fs/exfat/super.c`
- `/home/halifuda/linux/fs/exfat/misc.c`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYNC-31/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYNC-31/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYNC-31/03_designer_ktest.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- production code
- creator, checker, advisor, and reviewer artifacts for any component

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/designer.md`

## Semantic Prior Inputs

- Use the accepted architect boundary as authoritative.
- `ExfatFs` remains the only sync/flush-ordering owner.
- `ExfatInode` remains the owner of buffered writes, namespace mutation, and inode-visible dirty production.
- `write_page_async()` remains a downstream persistence seam and must not become a second page-cache owner.

## Integration Prior Inputs

- `fs.rs` still has a placeholder `sync()`.
- `inode.rs` still inherits default `sync_all()` / `sync_data()` and still rejects `write_page_async()`.
- `EXR-WRITE-30` and `EXR-NAMESPACE-29` define the current dirty producers this row must consume.
- The board now reserves `EXR-VOLLABEL-35`, `EXR-INODE-META-36`, and `EXR-BOOT-34` for later control-path or metadata surfaces; this design should allow them to feed the sync owner later without reopening the boundary.

## Workflow Prior Inputs

- Command-free designer lane.
- This is artifact-only planning and may overlap with the active `EXR-CHARSET-32` architect/designer line because the write sets are disjoint.
- Produce the standard split artifact set: core, async, and ktest.
- Stay designer-only; do not drift into creator implementation details beyond creator-ready obligations.

## Quality Prior Inputs

- Use the designer-role quality slice from `$exfat-subagent-workflow`.
- Keep helper shape explicitly owner-private to `ExfatFs`.
- Reject drift into a public writeback manager, control bucket, or inode-local sync owner.

## Temporary Interfaces And Exit Plan

- Do not authorize direct-I/O support, boot fallback policy, volume-label user control, trim/discard, or forced shutdown in this designer pass.
- If a temporary seam seems necessary, stop and report it instead of inventing one silently.

## Helper Justification

- Allowed helper surfaces are owner-private helpers that:
  - track or enumerate already-dirty filesystem/inode state,
  - order one sync call across filesystem-owned and inode-owned persistence steps,
  - and let inode sync hooks delegate into `ExfatFs` without becoming separate owners.
- They must remain subordinate to `ExfatFs`.

## Allowed Commands

- Read-only shell inspection commands under:
  - `/home/halifuda/asterinas`
  - `/home/halifuda/linux/fs/exfat`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with:
  - `EXR-CHARSET-32` architect/design planning

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYNC-31/01_designer_core.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYNC-31/02_designer_async.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYNC-31/03_designer_ktest.md`

## Escalation Rule

- If the architect artifact plus current upstream boundaries are still insufficient to specify sync and writeback ordering cleanly without reopening control-path policy or dirty-production ownership, report the exact missing handshake and stop instead of guessing.
