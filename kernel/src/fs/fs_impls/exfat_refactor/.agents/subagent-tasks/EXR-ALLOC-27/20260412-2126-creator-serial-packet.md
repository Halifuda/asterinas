<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-ALLOC-27-20260412-2126-CREATE-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-ALLOC-27/20260412-2126-creator-serial-packet.md`
- Supersedes: None
- Role: `creator`
- Component: `EXR-ALLOC-27`
- Phase: `serial creator`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 21:26 CST`

## Goal

- Implement the filesystem-owned allocator boundary so `ExfatFs` can search free space, keep reservation intent owner-private, commit bitmap and FAT changes as one owner-local sequence, and return the small committed result shape needed by later namespace and write rows.

## Architectural Unit Context

- Functional goal: `ExfatFs`-owned cluster allocation service
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-internal `Allocator` service plus owner methods and small owner-private helper surfaces
- Interfaces served:
  - later committed allocation consumption by `EXR-DENTRY-WRITE-28`
  - later data growth and write-side ownership under `EXR-WRITE-30`

## Required Resolution Questions

- Add the `Allocator` landing shape under `ExfatFs` without inventing a standalone free-space manager or inode-local allocator.
- Search the published allocation bitmap for a contiguous run first, then fall back to a FAT-backed fragmented result only when necessary.
- Keep reservation intent owner-private to one allocator call and publish only a small committed result shape carrying `start_cluster`, `cluster_count`, and `chain_mode`.
- Commit bitmap and FAT updates as one owner-local sequence without widening into namespace publication, file-size policy, truncate semantics, or sync ordering.
- If narrow owner-private mutation helpers are needed in `bitmap.rs` or `fat.rs`, keep them subordinate to allocator ownership and record them in the creator artifact.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/30_reviewer_report.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FATVAL-03A/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHAIN-03B/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/io.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/10_creator_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- sibling component artifacts

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/creator.md`

## Semantic Prior Inputs

- Use the accepted designer constraints only.
- `EXR-BITMAP-21` remains the owner of published bitmap snapshot facts. Any new mutation/search helper surface in `bitmap.rs` must exist only to serve the filesystem-owned allocator.
- `EXR-FATVAL-03A` and `EXR-CHAIN-03B` remain the FAT decode and chain-shape foundations. Do not turn this pass into a generic FAT service redesign.

## Integration Prior Inputs

- `allocator.rs` is the primary landing zone for allocator-owned search, reservation, and commit orchestration.
- `fs.rs` owns wiring, serialization, and publication of the allocator boundary under `ExfatFs`.
- Narrow owner-private mutation helpers in `bitmap.rs` and `fat.rs` are allowed only when they directly support allocator commit and remain hidden behind allocator ownership.
- Do not widen into `directory.rs`, namespace publication, sync ordering, or general metadata-write infrastructure outside the authorized files.

## Workflow Prior Inputs

- Command-free creator lane.
- This is the loop's only creator round.
- Do not run compile, test, format, Docker, KVM, or QEMU commands; checker will own executable verification.

## Quality Prior Inputs

- Use the creator-role quality slice from `$exfat-subagent-workflow`.
- Keep the stable result shape intentionally small and copyable.
- Record every new private helper, temporary seam, or local type that is added outside `allocator.rs` in the creator artifact together with its final owner or removal condition.

## Temporary Interfaces And Exit Plan

- Do not introduce a public reservation lease, background allocator worker, inode-local allocator, or sync/writeback manager.
- If a temporary helper is needed to bridge bitmap or FAT mutation, it must stay owner-private and name `ExfatFs` allocator ownership or the later row that absorbs it.
- If the landing still appears to require edits outside the authorized files, stop and report the missing handshake instead of widening scope.

## Helper Justification

- Allowed helper surfaces may:
  - scan one published bitmap snapshot for contiguous or fragmented free runs,
  - stage one owner-private reservation object for one allocator call,
  - and commit bitmap/FAT mutation in support of one published allocation result.
- They must remain subordinate to `ExfatFs` allocator ownership.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free production edit`
- May overlap with:
  - `EXR-PGCACHE-26` checker
  - artifact-only planning lanes with disjoint write sets
- Known conflicts:
  - `fs.rs`
  - `mod.rs`
  - `bitmap.rs`
  - `fat.rs`

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/10_creator_serial.md`

## Escalation Rule

- If allocator implementation still appears to require directory-write policy, inode growth/truncate policy, sync ordering, or edits outside the authorized files, report the exact missing handshake and stop instead of widening the component.
