<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DENTRY-WRITE-28-20260413-0806-CREATE-REPAIR`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DENTRY-WRITE-28/20260413-0806-creator-repair-unused-tail-packet.md`
- Supersedes: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DENTRY-WRITE-28/20260413-0752-creator-repair-growth-tail-packet.md`
- Role: `creator`
- Component: `EXR-DENTRY-WRITE-28`
- Phase: `serial creator repair`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 08:06 CST`

## Goal

- Finish the remaining `Unused`-tail correctness repair in `DirectoryEngine`:
  - whenever a write consumes an existing `Unused` terminator, publish a new correct terminator unless the record now reaches directory EOF,
  - and treat “record ends exactly at directory EOF” as a valid full-directory outcome rather than an error.

## Architectural Unit Context

- Functional goal: write-side directory-entry mutation in `DirectoryEngine`
- Final architectural owner: `ExfatFs` internal `DirectoryEngine`
- Expected landing form: owner-private write methods and helpers in `directory.rs`
- Prior creator artifacts:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/10_creator_serial.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/12_creator_serial_repair.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/14_creator_serial_repair.md`

## Required Resolution Questions

- Distinguish a fit that relies on an existing `Unused` tail from a fit that stays wholly inside deleted slots.
- In `place_dentry_set()` and relocation paths, if the placed record consumes an old `Unused` terminator, publish a new one after the record unless the record now ends exactly at directory EOF.
- In the in-place rewrite path, detect when expansion beyond `existing_slots` consumes an `Unused` terminator and publish a replacement terminator in that case.
- Preserve the current narrow owner boundary. Do not widen into namespace policy, allocator search, sync ordering, or inode metadata publication.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/14_creator_serial_repair.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/16_creator_serial_repair.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- sibling component artifacts except the required `16_creator_serial_repair.md`

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/creator.md`

## Semantic Prior Inputs

- `ExfatDentry::Unused` is a directory-stream terminator, but directory EOF is also a valid termination boundary when the logical stream is completely full.
- The correctness requirement is not “always write an `Unused` slot”; it is “never leave scan termination dependent on stale bytes”.
- A fit inside deleted slots should not invent a new terminator if later live entries still exist.

## Integration Prior Inputs

- Keep the repair in `directory.rs`.
- It is acceptable to enrich the owner-private placement result type with one explicit “consumes existing unused tail” fact if that is the narrowest implementation.
- It is acceptable to add one owner-private helper that inspects a small trailing reusable range for `Unused` consumption during in-place expansion.

## Workflow Prior Inputs

- Command-free creator repair lane.
- You are not alone in the codebase. Do not revert or overwrite others' edits; adjust to the current workspace state.
- Do not run compile, test, format, Docker, KVM, or QEMU commands.

## Quality Prior Inputs

- Use the creator-role quality slice from `$exfat-subagent-workflow`.
- Prefer one explicit owner-private fact for “consumed unused terminator” over scattered ad hoc rescans.
- Record the exact repair in the creator artifact, including the EOF no-op rule for terminator publication.

## Temporary Interfaces And Exit Plan

- Do not introduce a directory-tail manager or a broader scan service.
- Any new helper added for unused-tail detection must remain owner-private to `DirectoryEngine`.

## Helper Justification

- Allowed helper surfaces may:
  - carry whether a chosen placement consumes an existing `Unused` tail,
  - inspect a small reusable range during in-place expansion for `Unused` consumption,
  - publish a replacement terminator only when needed,
  - and treat exact-EOF placement as a valid no-op for terminator publication.

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

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/16_creator_serial_repair.md`

## Escalation Rule

- If fixing the remaining `Unused`-tail boundary requires edits outside `directory.rs` or a broader protocol change, report the exact missing handshake and stop.
