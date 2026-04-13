<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DENTRY-WRITE-28-20260413-0720-CREATE-REPAIR`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DENTRY-WRITE-28/20260413-0720-creator-repair-packet.md`
- Supersedes: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DENTRY-WRITE-28/20260413-0652-creator-serial-packet.md`
- Role: `creator`
- Component: `EXR-DENTRY-WRITE-28`
- Phase: `serial creator repair`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 07:20 CST`

## Goal

- Repair the just-landed `DirectoryEngine` write-side helpers so directory entry offsets remain logical offsets within the directory stream while all read/write/tombstone operations correctly traverse the current `ExfatChain`, including FAT-backed chains after growth.

## Architectural Unit Context

- Functional goal: write-side directory-entry mutation in `DirectoryEngine`
- Final architectural owner: `ExfatFs` internal `DirectoryEngine`
- Expected landing form: owner-private write methods and helpers in `directory.rs`
- Prior creator artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/10_creator_serial.md`

## Required Resolution Questions

- Keep `DirectoryRecordLocation` and `dentry_set_byte_offset` as logical directory-stream offsets that remain compatible with the accepted read-side `DirectoryEngine` contract.
- Repair helper reads and writes so they do not treat those logical offsets as block-device physical offsets.
- Ensure slot scanning, in-place rewrite, tombstoning, and committed-growth placement work on the current chain topology, including FAT-backed chains materialized after growth.
- If helper I/O now has to cross cluster boundaries, keep that chunking owner-private to `DirectoryEngine` rather than inventing a broader I/O service.
- Keep the repair narrow; do not widen into namespace policy, allocator search, sync ordering, or other files.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/io.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/12_creator_serial_repair.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- sibling component artifacts

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/creator.md`

## Semantic Prior Inputs

- `DirectoryRecordLocation` remains a logical directory-record location, not a physical media location.
- `DirectoryEngine` remains the owner of slot discovery and mutation mechanics.
- `ExfatChain` remains the source of cluster topology; do not collapse chain traversal into a fake contiguous media assumption.

## Integration Prior Inputs

- Limit the repair to `directory.rs`.
- Reuse `ExfatChain::walk_to_cluster_at_offset()` and related owner-local chain helpers when that keeps the repair narrow.
- Preserve the current creator pass shape unless a helper must be adjusted to restore logical-to-physical correctness.

## Workflow Prior Inputs

- Command-free creator repair lane.
- You are not alone in the codebase. Do not revert or overwrite others' edits; adjust to the current workspace state.
- Do not run compile, test, format, Docker, KVM, or QEMU commands.

## Quality Prior Inputs

- Use the creator-role quality slice from `$exfat-subagent-workflow`.
- Prefer one owner-private logical-offset-to-physical-chunk helper over repeated ad hoc conversions.
- Record every repaired helper surface and the reason it remains owner-private to `DirectoryEngine`.

## Temporary Interfaces And Exit Plan

- Do not introduce a general directory I/O service or broader metadata-write layer.
- If a new helper is needed to iterate logical directory bytes over a chain, it must remain owner-private to `DirectoryEngine`.

## Helper Justification

- Allowed helper surfaces may:
  - map one logical directory byte range onto one or more chain-local physical chunks,
  - read or write those chunks while preserving the logical directory-stream contract,
  - and keep tombstoning/placement local to `DirectoryEngine`.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free production edit`
- May overlap with:
  - command-free planning and review lanes only
- Known conflicts:
  - `directory.rs`

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/12_creator_serial_repair.md`

## Escalation Rule

- If restoring logical-to-physical correctness still requires edits outside `directory.rs` or a broader I/O abstraction, report the exact missing handshake and stop.
