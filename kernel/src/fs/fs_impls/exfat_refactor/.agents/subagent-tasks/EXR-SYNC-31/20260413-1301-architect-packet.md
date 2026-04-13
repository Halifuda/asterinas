<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-SYNC-31-20260413-1301-ARCHITECT`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-SYNC-31/20260413-1301-architect-packet.md`
- Supersedes: None
- Role: `architect`
- Component: `EXR-SYNC-31`
- Phase: `architect`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 13:01 CST`

## Goal

- Define the smallest owner-first `EXR-SYNC-31` unit that gives `ExfatFs` explicit ownership of persistence ordering and flush sequencing across exFAT dirty producers without absorbing boot fallback, direct I/O, volume-label control, inode metadata policy, or admin/control ioctls.

## Architectural Unit Context

- Functional goal: filesystem-owned sync / flush-ordering boundary
- Final architectural owner: `ExfatFs`
- Expected landing form: owner methods
- Board authority:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-POST28/00_architect.md`

## Required Resolution Questions

- What is the smallest architecturally real `ExfatFs` unit that covers:
  - VFS `FileSystem::sync()`
  - inode `sync_all()`
  - inode `sync_data()`
  - and the writeback side of `write_page_async()`
  without turning `EXR-SYNC-31` into a catch-all control row?
- Which dirty producers should this row explicitly consume now or later:
  - `EXR-WRITE-30`
  - `EXR-NAMESPACE-29`
  - `EXR-VOLLABEL-35`
  - `EXR-INODE-META-36`
  - maybe `EXR-BOOT-34` only where persistent boot flags are already dirty-state outputs rather than mount policy?
- What must stay out:
  - direct I/O contract
  - name conversion
  - boot fallback decision-making
  - volume-label user control flow
  - FAT-attribute ioctls
  - trim/discard
  - forced shutdown
- Where is the stable owner boundary between inode-visible sync hooks and filesystem-wide ordering?
- Should the row own only ordering and persistence of already-published dirty state, or also the collection/tracking boundary for which objects are dirty?

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/topaz-bridge-20260413-1256-tail-reshape.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-POST28/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/30_reviewer_report.md`
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

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYNC-31/00_architect.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- all production code
- all non-architect artifacts for any component

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/architect.md`

## Semantic Prior Inputs

- The post-tail audit already narrowed `EXR-SYNC-31` to flush ordering only; treat that as a scheduler-owned boundary constraint.
- Use the Linux files for writeback ordering and direct-I/O separation guidance, not to widen this row into Linux-specific ioctl parity.
- `EXR-WRITE-30` still leaves `write_page_async()` as the explicit downstream sync seam.

## Integration Prior Inputs

- `fs.rs` still implements placeholder `FileSystem::sync()`.
- `inode.rs` still inherits default `sync_all()` / `sync_data()` behavior and still has a placeholder `write_page_async()`.
- The board now plans `EXR-VOLLABEL-35` and `EXR-INODE-META-36`; architect this row so those future dirty producers can consume it without reopening its owner.
- Keep control-path policy out of this row: boot fallback, volume-label user control, `O_DIRECT`, and admin non-goals should remain elsewhere.

## Workflow Prior Inputs

- Command-free architect lane.
- This packet is for one architect artifact only.
- Recommend the stable unit boundary and future creator-slice guidance, but do not schedule or implement.

## Quality Prior Inputs

- Use the architect-role quality slice from `$exfat-subagent-workflow`.
- Reject "sync bucket" drift.
- Call out likely `fs.rs` / `inode.rs` collision zones for later creator waves.

## Temporary Interfaces And Exit Plan

- Do not edit `COMPONENT_INDEX.md`.
- Do not define designer-level lock ordering or test coverage yet.
- Stop after producing the architect artifact for `EXR-SYNC-31`.

## Helper Justification

- This row may recommend owner-private dirty-state helpers under `ExfatFs` if they are necessary for a real flush-order owner, but it must not create a separate writeback manager with its own public identity.

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
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYNC-31/00_architect.md`

## Escalation Rule

- If the current priors and Linux files still do not support a stable flush-order boundary without widening into control-path policy, report the exact unresolved split and stop instead of guessing.
