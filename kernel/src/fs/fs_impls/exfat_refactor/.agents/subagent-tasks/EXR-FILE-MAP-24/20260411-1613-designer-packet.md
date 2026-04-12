<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-FILE-MAP-24-20260411-1613-DESIGN`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FILE-MAP-24/20260411-1613-designer-packet.md`
- Supersedes: None
- Role: `designer`
- Component: `EXR-FILE-MAP-24`
- Phase: `designer`
- Authorizing main agent: `main-agent`
- Date: `2026-04-11 16:13 CST`

## Goal

- Produce the split designer artifact set for `EXR-FILE-MAP-24` so a later creator round can land `ExfatInode` logical-to-physical regular-file mapping helpers without guessing about owner boundary, helper shape, or checker obligations.

## Architectural Unit Context

- Functional goal: `ExfatInode` read-path logical-to-physical file mapping
- Final architectural owner: `ExfatInode`
- Expected landing form: owner-private helpers in `inode.rs`
- Architect artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/00_architect.md`

## Required Resolution Questions

- Specify the minimal owner-private helper set for regular-file logical-offset to physical-position mapping under `ExfatInode`.
- State how the row consumes accepted chain traversal, inode-owned size facts, and cluster geometry without absorbing actual data I/O or read policy.
- Keep directory traversal, mount/open sequencing, page-cache behavior, EOF and zero-fill policy, growth, and allocator mutation out of scope.
- Define narrow creator and checker obligations so later work does not guess where mapping stops and buffered read begins.
- State the repeated-call and local serialization expectations for helper use during later read-side callers.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_CORE_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_ASYNC_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_KTEST_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHAIN-03B/01_designer_spec.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/03_designer_ktest.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- production code
- creator, checker, advisor, and reviewer artifacts for any component

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/designer.md`

## Semantic Prior Inputs

- Use the accepted `EXR-FILE-MAP-24` architect boundary as authoritative for the split.
- Keep mapping subordinate to `ExfatInode`; do not promote `ExfatChain` or chain walking into a separate service owner.
- This row stops at address translation and physically mappable span derivation; EOF policy, zero-fill policy, and actual byte-copying belong later.

## Integration Prior Inputs

- `EXR-DIR-OPS-23` owns directory traversal and should stay separate from regular-file mapping even though both land under `ExfatInode`.
- `EXR-READ-OPS-25` owns later buffered read semantics and must remain the first owner of actual data transfer, short-read policy, and zero-fill behavior.
- `ExfatFs` remains the source of cluster geometry; this row consumes that context through the inode owner rather than becoming filesystem-global logic.

## Workflow Prior Inputs

- Command-free designer lane.
- This is artifact-only planning and may overlap with the active `EXR-DIR-OPS-23` creator lane because the write set is disjoint.
- Produce the standard split artifact set: core, async, and ktest.
- Stay designer-only; do not drift into creator implementation details beyond creator-ready obligations.

## Quality Prior Inputs

- Use the designer-role quality slice from `$exfat-subagent-workflow`.
- Keep helper shape explicitly owner-private to `ExfatInode`.
- Reject any design drift into standalone mapping service boundaries, read shells, or page-cache owner claims.

## Temporary Interfaces And Exit Plan

- Do not authorize a temporary mapping service, read shell, or data-path owner in this designer pass.
- If a temporary seam seems necessary, stop and report it instead of inventing one silently.

## Helper Justification

- Allowed helper surfaces are owner-private helpers that:
  - reconstruct or consume inode-owned chain facts,
  - translate a logical byte offset into chain position and in-cluster offset,
  - and derive the physically mappable span for a logical request.
- They must remain subordinate to `ExfatInode` and must not perform actual data reads, zero-fill, or allocation growth.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with:
  - the active `EXR-DIR-OPS-23` creator lane because the write set is artifact-only

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/01_designer_core.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/02_designer_async.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/03_designer_ktest.md`

## Escalation Rule

- If the architect artifact is still insufficient to specify a stable owner-private mapping helper set without reopening read policy, page-cache behavior, or directory logic, report the exact missing boundary and stop instead of guessing.
