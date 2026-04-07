<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-INODE-CACHE-18-20260407-1105-CREATE-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CACHE-18/20260407-1105-creator-serial-packet.md`
- Supersedes: None
- Role: `creator`
- Component: `EXR-INODE-CACHE-18`
- Phase: `serial implementation`
- Authorizing main agent: `main-agent`
- Date: `2026-04-07 11:05 CST`

## Goal

- Implement the `ExfatFs`-owned opened-inode table and owner-private `InodeKey` boundary from the accepted designer spec without widening into mount/open sequencing, directory traversal, or VFS root wiring.

## Architectural Unit Context

- Functional goal: opened-inode identity and cache ownership under `ExfatFs`, including a distinct root special case.
- Final architectural owner: `ExfatFs`.
- Expected landing form: owner-internal state plus validated `InodeKey` in `fs.rs`.
- Parent units: `EXR-FS-CORE-16`, `EXR-INODE-CORE-17`.
- Interfaces served: future `EXR-FS-OPEN-22`, later lookup/open reuse, and later VFS operations needing stable inode identity.

## Required Resolution Questions

- Add `InodeKey` as an owner-private value type derived only from trusted directory-location facts.
- Add the opened-inode table under `ExfatFs` and implement reuse-first lookup/publication and exact-key removal for non-root inodes.
- Reserve a root special-case slot that is not encoded as a synthetic ordinary `InodeKey`.
- Keep disk I/O, directory traversal, mount/open sequencing, and VFS root exposure out of this pass.
- Do not add helper shells or public field-exposing accessors without a designer-backed caller.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CREATOR_LOG_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/30_reviewer_report.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/30_reviewer_report.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- Read-only inspection commands inside `/home/halifuda/asterinas` are allowed.

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/10_creator_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Sibling component artifacts.

## Required Inputs

- Role-scoped protocol files: `COMMON_SUBAGENT.md`, `CREATOR.md`.
- Designer spec: `EXR-INODE-CACHE-18/01_designer_core.md`, `02_designer_async.md`, and `03_designer_ktest.md`.
- Creator log template.

## Semantic Prior Inputs

- Use accepted designer constraints and accepted value-type boundaries only. Do not reopen Linux behavior.

## Integration Prior Inputs

- Use the already-landed `ExfatFs` and `ExfatInode` surfaces, plus trusted `ExfatInodeLocation` facts if exposed by the reviewed carrier.

## Workflow Prior Inputs

- Command-free creator lane. Do not run compile, test, format, Docker, KVM, or QEMU commands.
- Launch only after `EXR-FS-CORE-16` and `EXR-INODE-CORE-17` reviewer passes are complete, because this pass edits `fs.rs` and depends on the reviewed inode carrier API.

## Quality Prior Inputs

- Use `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-CREATE`.
- Keep visibility narrow and avoid helper wrappers without designer-backed justification.

## Temporary Interfaces And Exit Plan

- Root publication may be represented as a reserved owner-private slot, but VFS `root_inode()` wiring remains in `EXR-FS-OPEN-22`.
- No new temporary staging surface is authorized unless it is owner-private and recorded with an exit condition in the creator log.

## Helper Justification

- `InodeKey` helpers are justified only by `ExfatFs` opened-inode table ownership.
- Do not expose cache internals as general-purpose helpers.

## Allowed Commands

- Read-only shell commands only.
- No build, test, format, Docker, KVM, or QEMU commands.

## Parallelism Classification

- Lane class: command-free production edit.
- Known conflicts: overlaps `fs.rs`; do not launch while `EXR-FS-CORE-16` reviewer or any other `fs.rs` writer is active.
- May overlap with `EXR-DIR-ENGINE-19` creator only if the sibling packet does not edit `fs.rs`.

## Execution Environment

- Host read-only inspection and file edits under `/home/halifuda/asterinas`.

## Execution Lock

- No execution lock is needed.

## Stop Condition

- Stop after implementing the assigned pass and writing `EXR-INODE-CACHE-18/10_creator_serial.md`.

## Escalation Rule

- If the cache boundary cannot be implemented without mount/open sequencing, directory traversal, or edits outside `fs.rs`, report the gap instead of widening scope.
