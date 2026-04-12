<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DIR-OPS-23-20260411-1627-CREATE-REPAIR`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-OPS-23/20260411-1627-creator-repair-packet.md`
- Supersedes:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-OPS-23/20260411-1613-creator-serial-packet.md`
- Role: `creator`
- Component: `EXR-DIR-OPS-23`
- Phase: `serial repair`
- Authorizing main agent: `main-agent`
- Date: `2026-04-11 16:27 CST`

## Goal

- Repair the blocked `EXR-DIR-OPS-23` creator return by landing the now-specified owner-facing handshake between `ExfatInode`, `DirectoryEngine`, and `ExfatFs`, and then implementing read-only `lookup` and `readdir_at` without widening into mutation, mount/open, or file-data behavior.

## Why This Repair Exists

- The first 2026-04-11 creator pass returned blocked with no production edits because the packet restricted work to `inode.rs` even though the accepted behavior still needed cross-owner bridges in `fs.rs` and trusted record-location projection from `directory.rs`.
- The repaired designer set now names those missing handshakes explicitly:
  - a filesystem-owned directory-stream bridge that starts a fresh `DirectoryEngine` for the current inode chain,
  - a filesystem-owned child-publication bridge that resolves or reuses the canonical child handle from trusted record-location facts,
  - and a directory-record projection that preserves the trusted location facts needed for `InodeKey`.
- Checker must not be used as a spec-completion lane, so this returns to creator as a narrow repair.

## Architectural Unit Context

- Functional goal: `ExfatInode` read-only directory operations
- Final architectural owner: `ExfatInode`
- Expected landing form: owner methods in `inode.rs`, consuming owner-facing bridges from `ExfatFs` and owner-internal directory-record projection from `DirectoryEngine`
- Interfaces served:
  - VFS `Inode::lookup`
  - VFS `Inode::readdir_at`
  - filesystem-owned child reuse and canonicalization already accepted under `ExfatFs`

## Required Resolution Questions

- Add the filesystem-owned directory-stream bridge that lets `inode.rs` obtain a fresh read-only `DirectoryEngine` for the current inode chain without exposing raw `ExfatFs` fields.
- Add the minimal trusted location projection needed for `lookup` to derive `InodeKey` from a matched file record.
- Add the filesystem-owned child-publication bridge that resolves or reuses the canonical child inode handle from those trusted location facts.
- Implement directory-only `lookup` and `readdir_at` on `ExfatInode` using the repaired designer contract.
- Keep trusted record-location facts owner-internal; do not expose them through VFS dirents or a new public service boundary.

## Read Set

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
- `/home/halifuda/asterinas/kernel/src/fs/utils/dirent_visitor.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/inode.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/12_creator_serial_repair.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- checker, reviewer, advisor, and handoff artifacts
- unrelated component artifact directories

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/creator.md`
- Implement against the repaired 2026-04-11 `EXR-DIR-OPS-23` designer set only; do not reopen architect or designer decisions unless the packet's escalation rule triggers.

## Semantic Prior Inputs

- The repaired `EXR-DIR-OPS-23` designer set is authoritative for this repair.
- The legacy exFAT inode path in the read set is semantic reference only for directory-op details and does not override the refactor's accepted owner boundaries.
- `lookup` remains read-only and must derive `InodeKey` from trusted record-location facts, not mutable inode metadata.
- `readdir_at` remains a projection of visible file records only and must keep trusted location facts owner-internal.

## Integration Prior Inputs

- `DirectoryEngine` remains an `ExfatFs`-owned read-only record-stream service; this repair may widen its owner-internal file-record projection only as needed to preserve trusted location facts for later consumers.
- `ExfatFs` remains the owner of canonicalization and opened-inode reuse. This repair may add owner-facing methods callable from `inode.rs`, but it must not expose raw `block_device`, `super_block`, or opened-inode state directly.
- `EXR-FS-OPEN-22` still owns mount/open and root publication; do not absorb those responsibilities here.
- `EXR-INODE-CACHE-18` still owns `InodeKey` and canonical child reuse; this repair must consume that boundary rather than recreating it in `inode.rs`.

## Workflow Prior Inputs

- Command-free creator repair lane.
- This is the only production creator lane currently allowed on `inode.rs`, `fs.rs`, and `directory.rs`.
- Do not run compile or test commands; checker will own executable verification.
- If the repaired spec still cannot land cleanly inside the packet write set, stop and report the exact remaining handshake instead of widening into more files.

## Quality Prior Inputs

- Use the creator-role quality slice from `$exfat-subagent-workflow`.
- Prefer owner methods or owner-private associated helpers over module-scope free functions when the final owner is already known.
- If you add a private helper, local type, accessor, or temporary seam, record it in the creator artifact together with its final owner or removal condition.
- Do not add overlapping helper APIs at multiple abstraction levels; choose one canonical owner-facing bridge per need.

## Temporary Interfaces And Exit Plan

- Do not introduce a separate lookup service, scanner shell, cache shell, or mutation shell.
- Do not introduce raw `ExfatFs` field accessors as a temporary shortcut.
- If a richer directory-record projection is needed, keep it owner-internal to `DirectoryEngine` and justified only as consumed output for `EXR-DIR-OPS-23`.

## Helper Justification

- Allowed new consumed-owner surfaces are:
  - an `ExfatFs` owner method callable from `inode.rs` that starts or returns a fresh read-only directory stream for a provided chain snapshot,
  - an `ExfatFs` owner method callable from `inode.rs` that resolves or publishes a canonical child inode from trusted record-location facts,
  - and a richer owner-internal file-record projection in `directory.rs` if that is the only way to preserve the trusted location facts required by `InodeKey`.
- Do not add field-exposing accessors or convenience helpers beyond those named needs.
- Do not let a helper become a floating module-level convenience seam or a new owner boundary.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free production edit`
- May overlap with:
  - artifact-only planning lanes with disjoint write sets
- Known conflicts:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`

## Execution Environment

- Host workspace only
- This task is command-free. Do not add compile or runtime commands on your own.

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/12_creator_serial_repair.md`.
- Do not proceed into checker work.

## Escalation Rule

- If the repaired designer set still leaves a real ambiguity about how trusted record-location facts flow to `InodeKey` or how canonical child publication remains filesystem-owned, report that exact ambiguity and stop instead of guessing.
