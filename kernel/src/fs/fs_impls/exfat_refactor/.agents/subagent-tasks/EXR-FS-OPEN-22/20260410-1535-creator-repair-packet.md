<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-FS-OPEN-22-20260410-1535-CREATE-REPAIR`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FS-OPEN-22/20260410-1535-creator-repair-packet.md`
- Supersedes:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FS-OPEN-22/20260410-1510-creator-serial-packet.md`
- Role: `creator`
- Component: `EXR-FS-OPEN-22`
- Phase: `serial repair`
- Authorizing main agent: `main-agent`
- Date: `2026-04-10 15:35 CST`

## Goal

- Repair the partial 22 creator return by adding the actual `ExfatFs` mount/open sequencing and prerequisite-order behavior that the designer required, while preserving the already-landed root-publication improvements.

## Why This Repair Exists

- The first creator return usefully removed the indefinite `root_inode()` seam and added a root-publication regression.
- However, it did **not** implement the actual owner-side mount/open sequence or the designer-required consumption order for opened-inode cache, `DirectoryEngine`, `UpcaseTable`, and `AllocationBitmap`.
- Checker must not be used as a spec-completion lane, so this returns to creator as a narrow repair.

## Architectural Unit Context

- Functional goal: `ExfatFs` mount/open sequencing and root publication
- Final architectural owner: `ExfatFs`
- Expected landing form: owner methods plus sequencing invariants in `fs.rs`

## Required Resolution Questions

- Add the real owner-side mount/open method or equivalent `fs.rs` sequence that turns ready prerequisites into a published root handle.
- Make the prerequisite order explicit in code, not only in comments or tests:
  - upcase readiness before name-sensitive discovery
  - allocation-bitmap readiness before publishing a ready root
  - root publication only after prerequisites and discovery facts are available
- Keep the root special case distinct from ordinary `InodeKey` entries.
- Preserve the useful root-publication regression and the removal of the old `todo!` seam if that remains compatible with the repaired sequence.
- Do not widen into later directory ops, namespace mutation, allocator mutation policy, data-path behavior, or sync ordering.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/12_creator_serial_repair.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Sibling component artifacts.

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/creator.md`

## Semantic Prior Inputs

- Use the accepted designer constraints only.
- `ExfatFs` remains the only mount/open owner.
- `DirectoryEngine` remains read-only.
- `UpcaseTable` remains canonicalization-only.
- `AllocationBitmap` remains read-only at this stage.

## Integration Prior Inputs

- Preserve the already landed root-publication slot and regression if compatible.
- Do not settle for a helper that merely allows tests to publish the root manually; the owner method must encode the sequencing contract itself.
- Keep this pass inside `fs.rs` only; if you discover the spec cannot be satisfied without edits to `directory.rs` or `inode.rs`, stop and report the exact missing handshake instead of widening scope.

## Workflow Prior Inputs

- Command-free creator lane.
- This remains the same loop's only creator round.
- Do not run compile or test commands; checker will own executable verification and local ktests.

## Quality Prior Inputs

- Use the creator-role quality slice from `$exfat-subagent-workflow`.
- Prefer small owner-private helpers in `fs.rs` that make sequencing and publication order explicit.

## Temporary Interfaces And Exit Plan

- The old `todo!` seam should not come back.
- Do not invent a separate mount object, root-scanner owner, or fake root carrier.
- Do not widen into later lookup/readdir, namespace mutation, allocator policy, page-cache, or sync logic.

## Helper Justification

- Small owner-private helpers in `fs.rs` are allowed if they encode prerequisite checks or root publication order clearly.
- Reject helpers whose main effect is to create a permanent manual-publication seam instead of the actual owner sequencing path.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free production edit`
- May overlap with command-free planning lanes only
- Known conflicts:
  - `fs.rs`

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/12_creator_serial_repair.md`

## Escalation Rule

- If the missing mount/open sequencing still cannot be implemented cleanly inside `fs.rs`, report the exact missing handshake and stop instead of widening scope.
