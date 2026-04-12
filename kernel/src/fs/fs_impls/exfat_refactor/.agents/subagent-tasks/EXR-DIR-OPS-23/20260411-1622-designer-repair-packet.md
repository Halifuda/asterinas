<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DIR-OPS-23-20260411-1622-DESIGN-REPAIR`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-OPS-23/20260411-1622-designer-repair-packet.md`
- Supersedes:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-OPS-23/20260410-1545-designer-packet.md`
- Role: `designer`
- Component: `EXR-DIR-OPS-23`
- Phase: `designer repair`
- Authorizing main agent: `main-agent`
- Date: `2026-04-11 16:22 CST`

## Goal

- Repair the `EXR-DIR-OPS-23` designer set so creator no longer has to guess about the cross-owner handshake between `ExfatInode`, `DirectoryEngine`, and `ExfatFs` when implementing read-only `lookup` and `readdir_at`.

## Why This Repair Exists

- The first 2026-04-11 creator lane stopped correctly under the packet escalation rule and recorded a real creator-blocking handshake gap in `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/10_creator_serial.md`.
- The current designer set assumes `inode.rs` can consume directory scanning and filesystem-owned child reuse directly, but the current implementation surface does not actually expose those owner-facing bridges.
- In addition, `DirectoryRecord::File(...)` currently carries only `ExfatDentrySet` and does not surface the primary-entry location facts needed for `InodeKey`-based opened-inode reuse.
- Checker must not be used as a spec-completion lane, and creator should not invent cross-owner bridges on its own, so this returns to designer as a narrow repair.

## Architectural Unit Context

- Functional goal: `ExfatInode` read-only directory operations
- Final architectural owner: `ExfatInode`
- Expected landing form: owner methods in `inode.rs`
- Architect artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/00_architect.md`

## Required Resolution Questions

- Name the exact owner-facing handshake that lets `inode.rs` consume `DirectoryEngine` without exposing raw `ExfatFs` fields.
- Name the exact owner-facing handshake that lets `lookup` resolve or publish a canonical child handle through the filesystem-owned opened-inode boundary.
- Decide what trusted location facts must be surfaced alongside file records so `lookup` can derive the validated location identity required by `InodeKey`.
- Keep those additions subordinate to existing accepted owners rather than turning them into a new lookup service, new scan owner, or new cache owner.
- Update the likely creator landing zones and write-set conflicts if the repaired spec now necessarily touches `inode.rs`, `fs.rs`, and `directory.rs`.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_CORE_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_ASYNC_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_KTEST_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/03_designer_ktest.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- production code
- creator, checker, advisor, and reviewer artifacts for any component other than the three repaired `EXR-DIR-OPS-23` designer files above

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/designer.md`

## Semantic Prior Inputs

- Keep `lookup` and `readdir_at` read-only and inode-owned.
- Keep `DirectoryEngine` read-only and owner-internal to `ExfatFs`.
- Keep `UpcaseTable` canonicalization-only and filesystem-owned.
- Keep opened-inode reuse filesystem-owned under `ExfatFs`.
- If a richer record shape is needed, it must still be justified as a consumed-owner output for later directory consumers, not as a new public service boundary.

## Integration Prior Inputs

- The blocked creator log is authoritative evidence for what the current spec still leaves ambiguous.
- `EXR-INODE-CACHE-18` requires trusted directory-location facts for `InodeKey`; the repaired spec must make clear where `lookup` gets those facts.
- `EXR-DIR-ENGINE-19` creator notes already anticipated that later integration might need a more specialized consumer-facing wrapper around `DirectoryRecord`; use that as a narrow integration clue rather than as permission to widen the owner boundary.
- `EXR-FS-OPEN-22` still owns mount/open and root publication; the repaired spec must not drift back into those responsibilities.

## Workflow Prior Inputs

- Command-free designer repair lane.
- Repair the existing split designer set in place; do not create a second parallel designer artifact family.
- Keep the repair bounded to `EXR-DIR-OPS-23`; do not reopen the live board or redesign `EXR-DIR-ENGINE-19` as a separate row.
- Make the likely creator write-set collision explicit if the repaired implementation now necessarily touches `inode.rs`, `fs.rs`, and `directory.rs`.

## Quality Prior Inputs

- Use the designer-role quality slice from `$exfat-subagent-workflow`.
- Do not authorize raw field-exposing accessors for `ExfatFs` internals such as `block_device`, `super_block`, or `opened_inode_state`.
- For every new helper or bridge you specify, name the expected caller and why that bridge is needed now.

## Temporary Interfaces And Exit Plan

- Do not authorize a separate lookup service, scanner shell, cache shell, or mutation shell.
- Do not authorize a raw `ExfatFs` field accessor as a temporary shortcut.
- If a richer directory-record projection is needed, keep it owner-internal to the directory stream and justified only as the consumer-facing shape needed by `EXR-DIR-OPS-23`.

## Helper Justification

- Allowed new consumed-owner surfaces may include:
  - an `ExfatFs` owner method callable from `inode.rs` that opens or drives a read-only directory stream for a provided chain snapshot,
  - an `ExfatFs` owner method callable from `inode.rs` that resolves or publishes a canonical child inode from trusted record-location facts,
  - and a richer owner-internal file-record output shape from `DirectoryEngine` if that is the only way to preserve the trusted location facts required by `InodeKey`.
- Do not authorize overlapping helper APIs that expose the same owner state at different abstraction levels.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with:
  - no active production creator lane on `inode.rs`, because the blocked creator has already returned
- Known conflicts:
  - no production-code writes are allowed in this lane

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after repairing:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/02_designer_async.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/03_designer_ktest.md`

## Escalation Rule

- If the blocked creator evidence means the stable unit boundary itself is wrong rather than just underspecified, report that exact boundary problem and stop instead of silently redesigning the component.
