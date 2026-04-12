<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-PGCACHE-26-20260412-1202-DESIGN`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-26/20260412-1202-designer-packet.md`
- Supersedes: None
- Role: `designer`
- Component: `EXR-PGCACHE-26`
- Phase: `designer`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 12:02 CST`

## Goal

- Produce the split designer artifact set for `EXR-PGCACHE-26` so later creator work can add inode-local `PageCache` ownership and `PageCacheBackend` integration on `ExfatInode` without re-owning buffered read semantics, write-side dirty policy, or a filesystem-global cache service.

## Architectural Unit Context

- Functional goal: inode-local page-cache integration under `ExfatInode`
- Final architectural owner: `ExfatInode`
- Expected landing form: owner-internal state plus trait impl in `inode.rs`
- Architect artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/00_architect.md`

## Required Resolution Questions

- Refine the architected inode-local cache boundary into a creator-ready spec without reopening the owner question.
- State what `PageCache` state and `PageCacheBackend` surface are architecturally real now.
- Define how cache population consumes the buffered-read owner from `EXR-READ-OPS-25` without duplicating EOF, short-read, or valid-size zero-fill policy.
- State what the row does with `write_page_async` and dirty/writeback concerns while `EXR-WRITE-30` and `EXR-SYNC-31` remain future owners.
- Decide whether a dedicated async artifact is required; if so, pin the per-call and per-page sequencing rules clearly.
- Define checker-owned test obligations for cache attachment, backend behavior, and read-only page-fill policy.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_CORE_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_ASYNC_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_KTEST_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/30_reviewer_report.md`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/page_cache.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/inode.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/03_designer_ktest.md`

## Forbidden Files

- production code
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- creator, checker, advisor, and reviewer artifacts for any component

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/designer.md`
- Based-on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/00_architect.md`

## Semantic Prior Inputs

- `EXR-READ-OPS-25` remains the first owner of buffered byte transfer. Cache page fill must consume that boundary rather than replace it.
- `EXR-FILE-MAP-24` remains translation-only and should appear here only as an already-accepted lower-layer dependency.
- Page-cache ownership is inode-local, not filesystem-global.

## Integration Prior Inputs

- `kernel/src/fs/vfs/page_cache.rs` is the authoritative trait and cache-container contract for this row.
- Legacy `kernel/src/fs/fs_impls/exfat/inode.rs` is orientation material for page-cache shape only. It does not override the current owner-first boundary.
- Because `EXR-WRITE-30` and `EXR-SYNC-31` are future owners, any temporary unsupported behavior around dirty/writeback surfaces must be named explicitly rather than hand-waved.

## Workflow Prior Inputs

- Command-free designer lane.
- This lane may overlap with the active `EXR-READ-OPS-25` creator lane and the `EXR-ALLOC-27` designer lane because the write set is artifact-only and disjoint.
- Produce the standard split artifact set: core, async, and ktest.
- Stay designer-only; do not drift into creator implementation details beyond creator-ready obligations.

## Quality Prior Inputs

- Use the designer-role quality slice from `$exfat-subagent-workflow`.
- Keep the cache boundary subordinate to `ExfatInode`.
- Reject drift into a cache manager service, duplicate buffered-read shell, or early writeback policy.

## Temporary Interfaces And Exit Plan

- Do not authorize a filesystem-global cache service, public cache manager, or dirty writeback owner in this designer pass.
- If the design needs a temporary unsupported backend behavior because write-side ownership does not exist yet, name the exact future owner that will absorb it.

## Helper Justification

- Allowed helper surfaces are owner-private helpers that:
  - attach cache state to `ExfatInode`,
  - bridge cache page fill through the existing buffered-read owner,
  - and account for page count or cache sizing without taking on growth/truncate policy.
- They must remain subordinate to inode-local cache ownership.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with:
  - `EXR-READ-OPS-25` creator
  - `EXR-ALLOC-27` designer

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/01_designer_core.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/02_designer_async.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/03_designer_ktest.md`

## Escalation Rule

- If the architect artifact plus current buffered-read design are still insufficient to specify stable page-cache ownership without deciding write-side growth, dirty persistence, or a duplicate read-policy shell, report the exact missing handshake and stop instead of guessing.
