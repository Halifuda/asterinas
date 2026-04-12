<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-READ-OPS-25-20260412-1110-DESIGN`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-READ-OPS-25/20260412-1110-designer-packet.md`
- Supersedes: None
- Role: `designer`
- Component: `EXR-READ-OPS-25`
- Phase: `designer`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 11:10 CST`

## Goal

- Produce the split designer artifact set for `EXR-READ-OPS-25` so a later creator round can land buffered regular-file `read_at` behavior on `ExfatInode` without guessing about mapping consumption, EOF and short-read ownership, valid-size zero-fill policy, or later page-cache boundaries.

## Architectural Unit Context

- Functional goal: buffered regular-file `read_at` on `ExfatInode`
- Final architectural owner: `ExfatInode`
- Expected landing form: owner methods in `inode.rs`
- Architect artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/00_architect.md`

## Required Resolution Questions

- Specify the smallest owner-method buffered-read path under `ExfatInode`.
- State exactly how `EXR-READ-OPS-25` consumes the mapping output from `EXR-FILE-MAP-24` without reopening mapping ownership.
- Define where EOF truncation, short-read return behavior, and valid-size zero-fill policy belong.
- Keep directory behavior, page-cache ownership, write-side growth, truncate, allocator mutation, and sync ordering out of scope.
- Define narrow creator and checker obligations so later work does not guess where mapping stops and buffered read begins.
- State the serialization and repeated-call expectations for the read path without inventing a new cache owner.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_CORE_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_ASYNC_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_KTEST_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/03_designer_ktest.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- production code
- creator, checker, advisor, and reviewer artifacts for any component

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/designer.md`

## Semantic Prior Inputs

- Use the accepted `EXR-READ-OPS-25` architect boundary as authoritative.
- `EXR-FILE-MAP-24` owns translation only. Buffered byte transfer, EOF, short-read, and valid-size zero-fill ownership begin here.
- `EXR-PGCACHE-26` remains a later owner. Do not let buffered read turn into page-cache ownership or a generic reader service.

## Integration Prior Inputs

- The current `EXR-FILE-MAP-24` creator artifact is part of the dependency contract for this designer pass, including the temporary explicit traversal-context arguments. Consume that current shape without turning it into a permanent second owner.
- `InodeIo::read_at` is the stable entry point to absorb the current seam.
- Keep directory behavior, write-side mutation, allocator policy, and sync ordering out of scope.

## Workflow Prior Inputs

- Command-free designer lane.
- This is artifact-only planning and may overlap with the `EXR-FILE-MAP-24` checker lane because the write set is disjoint.
- Produce the standard split artifact set: core, async, and ktest.
- Stay designer-only; do not drift into creator implementation details beyond creator-ready obligations.

## Quality Prior Inputs

- Use the designer-role quality slice from `$exfat-subagent-workflow`.
- Keep helper shape explicitly owner-private to `ExfatInode`.
- Reject drift into filesystem-global readers, page-cache shells, or write-side ownership.

## Temporary Interfaces And Exit Plan

- Do not authorize a public read service, page-cache shell, or write-side helper in this designer pass.
- If a temporary seam seems necessary, stop and report it instead of inventing one silently.

## Helper Justification

- Allowed helper surfaces are owner-private helpers that:
  - consume the mapping output from `EXR-FILE-MAP-24`,
  - decide one buffered-read slice and zero-fill extent,
  - and copy bytes into the caller-owned `VmWriter`.
- They must remain subordinate to `ExfatInode`.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with:
  - the active `EXR-FILE-MAP-24` checker lane because the write set is artifact-only

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/01_designer_core.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/02_designer_async.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/03_designer_ktest.md`

## Escalation Rule

- If the architect artifact plus current `EXR-FILE-MAP-24` contract are still insufficient to specify a stable buffered-read owner boundary without deciding page-cache or write-side ownership, report the exact missing handshake and stop instead of guessing.
