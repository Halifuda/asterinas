<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DIR-OPS-23-20260410-1510-ARCH`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-OPS-23/20260410-1510-architect-packet.md`
- Supersedes: None
- Role: `architect`
- Component: `EXR-DIR-OPS-23`
- Phase: `architect`
- Authorizing main agent: `main-agent`
- Date: `2026-04-10 15:10 CST`

## Goal

- Architect the stable `ExfatInode` directory-lookup and `readdir_at` unit that consumes the published root and `DirectoryEngine` without widening into mutation, allocation policy, or a fake directory-service owner.

## Architectural Unit Context

- Functional goal: implement read-only directory operations on `ExfatInode`
- Final architectural owner: `ExfatInode`
- Expected landing form: owner methods
- Boundary expectation from board reset: `DirectoryEngine` remains an owner-internal read-only service; lookup/readdir live on `ExfatInode`

## Required Resolution Questions

- Define the smallest functionally coherent read-only directory-operations unit under `ExfatInode`.
- State how the unit consumes accepted owners: published root / `ExfatInode`, `DirectoryEngine`, and `UpcaseTable`.
- State what still belongs outside this unit, especially namespace mutation, write-side directory entry updates, allocator policy, and data-path behavior.
- Make the dependency on the now-specified mount/open boundary explicit without turning mount/open sequencing into part of this row.
- Recommend dependency-safe work slices without inventing a separate lookup service or directory-scanner owner.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-RESET/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/00_architect.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- production code
- designer, creator, checker, advisor, and reviewer artifacts for other components

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/architect.md`

## Semantic Prior Inputs

- Use owner-first board-reset reasoning as authoritative for this split.
- The key question is stable owner convergence for read-only directory operations, not broad Linux parity beyond what the accepted owners already imply.

## Integration Prior Inputs

- `DirectoryEngine` is already the accepted read-only record stream and must not be reintroduced as a user-facing lookup owner.
- `EXR-FS-OPEN-22` owns mount/open sequencing and root publication; this row should consume the published root rather than absorb that responsibility.
- `UpcaseTable` remains the name-folding prerequisite for name-sensitive lookup behavior.

## Workflow Prior Inputs

- Command-free architect lane.
- This is parallel pre-research and must remain architect-only; do not drift into designer or creator detail.

## Quality Prior Inputs

- Use architect-level boundary guidance only.

## Temporary Interfaces And Exit Plan

- Temporary seams may be referenced only to explain how `EXR-DIR-OPS-23` begins after root publication.
- Do not authorize a separate lookup service, scanner owner, or mutation shell in this architect pass.

## Helper Justification

- Any helper-like surface proposed here must be justified as owner-internal to `ExfatInode` or `DirectoryEngine`, and still subordinate to the stable final owner.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with the single creator round because the write set is artifact-only

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/00_architect.md`

## Escalation Rule

- If accepted prior artifacts are still insufficient to name a stable owner-first directory-ops unit, report the exact missing handshake and stop instead of inventing a staging owner.
