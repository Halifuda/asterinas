<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-READ-OPS-25-20260412-1018-ARCH`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-READ-OPS-25/20260412-1018-architect-packet.md`
- Supersedes: None
- Role: `architect`
- Component: `EXR-READ-OPS-25`
- Phase: `architect`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 10:18 CST`

## Goal

- Architect the stable `ExfatInode` buffered `read_at` unit that will consume inode-owned mapping output from `EXR-FILE-MAP-24` without widening into page-cache ownership, write-side growth, or a fake read-service boundary.

## Architectural Unit Context

- Functional goal: buffered `read_at` semantics for regular-file `ExfatInode`
- Final architectural owner: `ExfatInode`
- Expected landing form: owner methods
- Boundary expectation from board reset:
  - `EXR-FILE-MAP-24` owns translation only
  - `EXR-READ-OPS-25` becomes the first owner of actual read-side byte transfer and short-read policy
  - `EXR-PGCACHE-26` stays a later, separate cache-integration row

## Required Resolution Questions

- Define the smallest functionally coherent buffered-read unit under `ExfatInode` once mapping output exists.
- State how `EXR-READ-OPS-25` consumes `EXR-FILE-MAP-24` mapping helpers without absorbing them into a second owner.
- State where actual data transfer, short-read behavior, EOF handling, and valid-size zero-fill policy should live.
- State what must remain outside this row, especially page-cache ownership, write-side growth, truncate, allocator mutation, and sync ordering.
- Make the dependency on the current `read_at` temporary seam explicit without turning that seam into a fake long-lived staging boundary.
- Recommend dependency-safe work slices without inventing a filesystem-global read service.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-RESET/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/00_architect.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- production code
- designer, creator, checker, advisor, and reviewer artifacts for any component

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/architect.md`

## Semantic Prior Inputs

- Use owner-first board-reset reasoning as authoritative for this split.
- `EXR-FILE-MAP-24` owns logical-to-physical translation only; do not fold mapping ownership back into `EXR-READ-OPS-25`.
- `EXR-READ-OPS-25` is the first row allowed to own actual buffered read behavior, including short-read and zero-fill policy decisions.
- `EXR-PGCACHE-26` remains a separate later owner; do not pre-merge cache integration here.

## Integration Prior Inputs

- `EXR-INODE-CORE-17` already owns the current `InodeIo::read_at` seam in `inode.rs`; this row should absorb that seam next rather than inventing a second read carrier.
- Keep directory behavior, mount/open sequencing, namespace mutation, and write-side allocation out of scope.
- This packet is pre-research while `EXR-FILE-MAP-24` creator is active. Use the accepted `24` architect/designer set as the current dependency contract; do not assume unstated implementation details from the in-flight creator lane.

## Workflow Prior Inputs

- Command-free architect lane.
- This lane may overlap with the active `EXR-FILE-MAP-24` creator round because the write set is artifact-only and disjoint.
- Stay architect-only; do not drift into designer obligations or production edit plans beyond boundary-safe work-slice recommendations.

## Quality Prior Inputs

- Use architect-level boundary guidance only.

## Temporary Interfaces And Exit Plan

- Temporary seams may be referenced only to explain how `EXR-READ-OPS-25` absorbs the existing `read_at` placeholder under `ExfatInode`.
- Do not authorize a public read service, filesystem-global read helper, or page-cache shell in this architect pass.

## Helper Justification

- Any helper-like surface proposed here must remain subordinate to `ExfatInode` and justified by buffered-read ownership, not by packet convenience.
- If a proposed helper starts to look like cache ownership, write-side growth ownership, or filesystem-global I/O ownership, reject it and keep that work in its later row.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with:
  - the active `EXR-FILE-MAP-24` creator lane because the write set is artifact-only

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/00_architect.md`

## Escalation Rule

- If accepted prior artifacts are still insufficient to name a stable owner-first buffered-read unit without prematurely deciding page-cache, write-side growth, or global I/O ownership, report the exact missing handshake and stop instead of inventing a staging owner.
