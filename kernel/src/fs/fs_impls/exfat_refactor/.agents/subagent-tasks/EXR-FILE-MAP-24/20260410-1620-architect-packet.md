<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-FILE-MAP-24-20260410-1620-ARCH`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FILE-MAP-24/20260410-1620-architect-packet.md`
- Supersedes: None
- Role: `architect`
- Component: `EXR-FILE-MAP-24`
- Phase: `architect`
- Authorizing main agent: `main-agent`
- Date: `2026-04-10 16:20 CST`

## Goal

- Architect the stable `ExfatInode` logical-to-physical file-mapping unit that supports later read-side file access without widening into data I/O, page-cache behavior, allocation growth, or write-side mutation.

## Architectural Unit Context

- Functional goal: regular-file logical-to-physical mapping for the read path
- Final architectural owner: `ExfatInode`
- Expected landing form: owner-private helpers
- Boundary expectation from board reset: read-path mapping remains subordinate to the inode owner and should not become a separate mapping service

## Required Resolution Questions

- Define the smallest functionally coherent mapping unit under `ExfatInode`.
- State how the unit consumes accepted owners and value types such as `ExfatChain`, validated inode state, and file-size / cluster-size facts.
- State what still belongs outside this unit, especially actual data I/O, page-cache integration, write-side growth, allocator policy, and sync ordering.
- Recommend dependency-safe work slices without inventing a separate mapping service owner.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-RESET/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHAIN-03B/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/00_architect.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- production code
- designer, creator, checker, advisor, and reviewer artifacts for other components

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/architect.md`

## Semantic Prior Inputs

- Use owner-first board-reset reasoning as authoritative for this split.
- The key question is stable owner convergence for read-path mapping, not broad Linux parity beyond what accepted owners already imply.

## Integration Prior Inputs

- `EXR-DIR-OPS-23` owns directory traversal; this row should begin after a file inode has already been resolved.
- `ExfatChain` remains a consumed value/service boundary, not a new user-visible owner.
- `EXR-READ-OPS-25` will own actual buffered reads later, so this row must stop at logical-to-physical mapping support.

## Workflow Prior Inputs

- Command-free architect lane.
- This is parallel pre-research and must remain architect-only; do not drift into designer or creator detail.

## Quality Prior Inputs

- Use architect-level boundary guidance only.

## Temporary Interfaces And Exit Plan

- Do not authorize a separate mapping service, read shell, or data-path owner in this architect pass.

## Helper Justification

- Any helper-like surface proposed here must be justified as owner-private to `ExfatInode` and still subordinate to the stable final owner.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with the checker lane because the write set is artifact-only

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/00_architect.md`

## Escalation Rule

- If accepted prior artifacts are still insufficient to name a stable owner-first file-mapping unit, report the exact missing handshake and stop instead of inventing a staging owner.
