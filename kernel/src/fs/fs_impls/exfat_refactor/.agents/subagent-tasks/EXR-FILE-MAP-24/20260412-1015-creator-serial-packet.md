<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-FILE-MAP-24-20260412-1015-CREATE-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FILE-MAP-24/20260412-1015-creator-serial-packet.md`
- Supersedes: None
- Role: `creator`
- Component: `EXR-FILE-MAP-24`
- Phase: `serial creator`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 10:15 CST`

## Goal

- Implement the `ExfatInode`-private logical-to-physical regular-file mapping helpers in `inode.rs` so later read-side owners can translate a logical file offset into a backed on-disk position and physically mappable span without widening into buffered read policy, zero-fill, or data copying.

## Architectural Unit Context

- Functional goal: `ExfatInode` read-path logical-to-physical file mapping
- Final architectural owner: `ExfatInode`
- Expected landing form: owner-private helpers in `inode.rs`
- Interfaces served:
  - later `EXR-READ-OPS-25`
  - later `EXR-PGCACHE-26`
  - the existing temporary `read_at` seam in `ExfatInode`

## Required Resolution Questions

- Land the smallest owner-private helper set in `inode.rs` that:
  - translates a regular-file logical byte offset into the containing chain position and in-cluster byte offset,
  - and derives the maximal physically mappable span for one logical request.
- Consume inode-owned chain facts, size facts, and filesystem cluster geometry without promoting `ExfatChain` into a standalone mapping service.
- Keep the result shape subordinate to `ExfatInode`; a small private return type is acceptable only if it keeps later read-side callers from guessing.
- Preserve the separation between this row and later buffered-read policy: do not decide EOF behavior, valid-size zero-fill behavior, or short-read policy here.
- Do not widen into directory behavior, mount/open sequencing, page-cache ownership, allocator mutation, or write-side growth.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHAIN-03B/01_designer_spec.md`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/10_creator_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- checker, reviewer, advisor, and handoff artifacts

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/creator.md`
- Implement against the accepted `EXR-FILE-MAP-24` architect/designer artifacts only; do not import unrelated component history unless the escalation rule triggers.

## Semantic Prior Inputs

- Use the accepted `EXR-FILE-MAP-24` designer set as authoritative for behavior and boundary.
- Mapping remains subordinate to `ExfatInode`; `ExfatChain` stays the accepted traversal boundary, not a new mapping owner.
- This row stops at translation plus physically mappable span derivation. It must not claim byte-copying, EOF policy, valid-size zero-fill policy, or short-read ownership.

## Integration Prior Inputs

- `EXR-INODE-CORE-17` already established the explicit temporary `read_at` seam. Keep that seam explicit in this pass; do not replace it with buffered-read behavior.
- `ExfatFs` remains the source of cluster geometry only. Consume that context through the inode owner rather than shifting logic into `fs.rs`.
- The current loop's directory work is already outside this packet. Do not reopen `lookup`, `readdir_at`, or any directory-stream owner questions here.

## Workflow Prior Inputs

- Command-free creator lane.
- This is the loop's only production creator round.
- Keep the landing inside `inode.rs`; if the accepted designer behavior cannot land cleanly there, stop and report the exact missing handshake instead of widening into another file.
- Do not run compile or test commands; later verification, if needed, is not part of this packet.

## Quality Prior Inputs

- Use the creator-role quality slice from `$exfat-subagent-workflow`.
- Keep any helper surface owner-private to `ExfatInode`.
- Prefer a small cluster of owner-private helpers or one small private result type over a fake read service or free helper module.
- If you add a private helper, local type, or temporary seam, record it in the creator artifact together with its final owner or removal condition.

## Temporary Interfaces And Exit Plan

- An owner-private mapping result type is allowed if it stays local to `inode.rs` and exists only to keep later read owners from recomputing translation facts.
- Do not introduce a separate mapping service, read shell, or page-cache-facing owner in this pass.
- Do not alter `read_at()` beyond preserving its explicit temporary seam for later owners.

## Helper Justification

- Allowed owner-private helpers may:
  - reconstruct or consume inode-owned chain facts,
  - translate a logical offset into chain position and in-cluster offset,
  - and derive the maximal physically backed span for one logical request.
- Do not add field-exposing accessors or cross-owner convenience helpers unless they are directly required by one of the above in this pass.
- Do not let a helper quietly become the first buffered-read owner in practice.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free production edit`
- May overlap with:
  - command-free planning lanes with disjoint write sets
- Known conflicts:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - later checker or reviewer lanes for `EXR-FILE-MAP-24`

## Execution Environment

- Host workspace only
- This task is command-free. Do not add compile or runtime commands on your own.

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/10_creator_serial.md`.
- Do not proceed into checker work.

## Escalation Rule

- If implementing the accepted designer behavior appears to require edits outside `inode.rs`, or if the only way to finish would be to decide buffered-read, zero-fill, EOF, or page-cache policy, report the exact missing handshake and stop instead of widening scope.
