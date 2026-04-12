<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-PGCACHE-26-20260412-2049-CREATE-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-26/20260412-2049-creator-serial-packet.md`
- Supersedes: None
- Role: `creator`
- Component: `EXR-PGCACHE-26`
- Phase: `serial creator`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 20:49 CST`

## Goal

- Implement the inode-local page-cache boundary on `ExfatInode` so `inode.rs` owns `PageCache` attachment and `PageCacheBackend` integration while consuming the accepted buffered-read owner from `EXR-READ-OPS-25` and keeping dirty persistence explicitly deferred.

## Architectural Unit Context

- Functional goal: inode-local page-cache integration under `ExfatInode`
- Final architectural owner: `ExfatInode`
- Expected landing form: owner-internal state plus trait impl in `inode.rs`
- Interfaces served:
  - `PageCacheBackend`
  - inode-local cached page population for later file I/O
  - accepted buffered regular-file read behavior from `EXR-READ-OPS-25`

## Required Resolution Questions

- Add inode-local `PageCache` ownership to `ExfatInode` without inventing a filesystem-global cache service.
- Land the `PageCacheBackend` impl on the inode carrier itself.
- Populate cache pages by delegating through the accepted buffered-read owner rather than rebuilding EOF, short-read, or valid-size zero-fill policy.
- Derive page-count accounting from the inode snapshot.
- Keep `write_page_async` as a narrow future-owned surface instead of inventing dirty writeback policy.
- Do not widen into allocator ownership, write-side growth, truncate, namespace mutation, or sync ordering.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/30_reviewer_report.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/page_cache.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/inode.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/10_creator_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/`
- sibling component artifacts

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/creator.md`
- Implement against the accepted `EXR-PGCACHE-26` designer set only; do not reopen architect or designer decisions unless the packet's escalation rule triggers.

## Semantic Prior Inputs

- `EXR-READ-OPS-25` remains the only owner of buffered byte-stream policy. Cache page fill must consume that owner boundary.
- `EXR-FILE-MAP-24` remains translation-only and should stay subordinate.
- `write_page_async` is structurally required by the trait, but dirty persistence is still future-owned by `EXR-WRITE-30` / `EXR-SYNC-31`.

## Integration Prior Inputs

- Reuse the generic `PageCache` / `PageCacheBackend` contract from `page_cache.rs`; do not invent a second cache layer.
- Legacy `kernel/src/fs/fs_impls/exfat/inode.rs` is reference material for owner shape only and does not override the current accepted read-owner boundary.
- If a narrow `mod.rs` visibility/import adjustment is needed, keep it minimal and record it in the creator artifact.

## Workflow Prior Inputs

- Command-free creator lane.
- This is the next production creator wave after `EXR-READ-OPS-25` acceptance.
- Do not run compile, test, format, Docker, KVM, or QEMU commands; checker will own executable verification.

## Quality Prior Inputs

- Use the creator-role quality slice from `$exfat-subagent-workflow`.
- Keep the cache boundary owner-private to `ExfatInode`.
- Record any temporary unsupported `write_page_async` behavior or constructor seam explicitly in the creator artifact together with its later owner/removal condition.

## Temporary Interfaces And Exit Plan

- A temporary unsupported `write_page_async` path is allowed only if it is explicitly marked as future-owned by `EXR-WRITE-30` / `EXR-SYNC-31`.
- Do not add a filesystem-global cache service, dirty writeback helper, or duplicate buffered-read shell.

## Helper Justification

- Allowed owner-private helpers may:
  - construct and attach the inode-local `PageCache`,
  - bridge one page fill through the existing buffered-read owner,
  - and derive page-count/capacity facts from the inode snapshot.
- They must remain subordinate to `ExfatInode`.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free production edit`
- May overlap with:
  - artifact-only planning lanes with disjoint write sets
- Known conflicts:
  - `inode.rs`
  - `mod.rs`
  - later checker or reviewer lanes for `EXR-PGCACHE-26`

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/10_creator_serial.md`

## Escalation Rule

- If the accepted designer behavior still appears to require edits outside `inode.rs` and the allowed narrow `mod.rs` adjustment, or if implementation would require write-side, sync, allocator, or cache-manager decisions, report the exact missing handshake and stop instead of widening scope.
