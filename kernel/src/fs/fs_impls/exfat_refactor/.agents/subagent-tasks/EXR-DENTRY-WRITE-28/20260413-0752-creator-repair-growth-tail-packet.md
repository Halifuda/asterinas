<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DENTRY-WRITE-28-20260413-0752-CREATE-REPAIR`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DENTRY-WRITE-28/20260413-0752-creator-repair-growth-tail-packet.md`
- Supersedes: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DENTRY-WRITE-28/20260413-0720-creator-repair-packet.md`
- Role: `creator`
- Component: `EXR-DENTRY-WRITE-28`
- Phase: `serial creator repair`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 07:52 CST`

## Goal

- Repair the `DirectoryEngine` growth path so directory writes remain reachable and scan-safe after committed growth:
  - growth must continue from the earliest reusable logical tail slot, not blindly from the old allocation end,
  - and the post-write directory stream must still expose a correct `Unused` termination boundary rather than stale media bytes.

## Architectural Unit Context

- Functional goal: write-side directory-entry mutation in `DirectoryEngine`
- Final architectural owner: `ExfatFs` internal `DirectoryEngine`
- Expected landing form: owner-private write methods and helpers in `directory.rs`
- Prior creator artifacts:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/10_creator_serial.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/12_creator_serial_repair.md`

## Required Resolution Questions

- Preserve the logical-directory-stream semantics already restored in the last repair.
- When `find_reusable_slot_run()` reaches an `Unused` terminator but the existing allocated tail is too short, extend the directory and place the new record at that same logical tail start rather than at the old `directory_entry_count()`.
- When growth occurs with no prior `Unused` terminator, ensure the newly visible tail still contains a valid `Unused` stop marker after the placed record.
- Do not leave newly visible directory space dependent on uninitialized old media contents for scan termination.
- Keep the repair narrow and owner-private to `DirectoryEngine`; do not widen into namespace policy, allocator search, sync ordering, or inode metadata publication.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/12_creator_serial_repair.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/14_creator_serial_repair.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- sibling component artifacts except the required `14_creator_serial_repair.md`

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/creator.md`

## Semantic Prior Inputs

- `DirectoryEngine` owns slot discovery and on-disk directory mutation, but it does not own namespace policy or allocator search.
- `ExfatDentry::Unused` is the logical directory-stream terminator; later scans must not depend on stale bytes beyond the written record.
- A committed `AllocationResult` remains only a growth fact. This repair must not widen into allocation reservation or search.

## Integration Prior Inputs

- Limit the repair to `directory.rs`.
- It is acceptable to replace the current placement search helper with a small owner-private result type if that is the narrowest way to distinguish:
  - reusable space already sufficient now,
  - reusable tail space that becomes sufficient only after growth,
  - and no reusable tail before the old allocation end.
- It is acceptable to add one owner-private helper that writes an `Unused` terminator or zero-fills newly visible tail bytes, provided it remains subordinate to `DirectoryEngine`.

## Workflow Prior Inputs

- Command-free creator repair lane.
- You are not alone in the codebase. Do not revert or overwrite others' edits; adjust to the current workspace state.
- Do not run compile, test, format, Docker, KVM, or QEMU commands.

## Quality Prior Inputs

- Use the creator-role quality slice from `$exfat-subagent-workflow`.
- Prefer one explicit owner-private placement result over ad hoc re-scanning if that keeps the growth logic easy to verify.
- Record exactly how the repaired growth path preserves reachability and `Unused` termination semantics.

## Temporary Interfaces And Exit Plan

- Do not introduce a general directory allocator, write manager, or metadata publication service.
- Any new helper added for growth-tail placement or `Unused` publication must remain owner-private to `DirectoryEngine`.

## Helper Justification

- Allowed helper surfaces may:
  - remember the logical slot where growth should continue,
  - place a validated record after growth at the correct logical tail start,
  - publish an `Unused` terminator or equivalent zero-visible tail condition after the new record,
  - and keep those details internal to `DirectoryEngine`.

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

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/14_creator_serial_repair.md`

## Escalation Rule

- If restoring growth-tail reachability and `Unused` termination requires widening into inode metadata publication, `fs.rs`, or a spec change outside the assigned write set, report the exact missing handshake and stop instead of guessing.
