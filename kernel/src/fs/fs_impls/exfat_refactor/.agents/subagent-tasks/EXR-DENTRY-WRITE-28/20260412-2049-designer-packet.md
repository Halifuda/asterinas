<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DENTRY-WRITE-28-20260412-2049-DESIGN`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DENTRY-WRITE-28/20260412-2049-designer-packet.md`
- Supersedes: None
- Role: `designer`
- Component: `EXR-DENTRY-WRITE-28`
- Phase: `designer`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 20:49 CST`

## Goal

- Produce the split designer artifact set for `EXR-DENTRY-WRITE-28` so later creator work can land directory-entry mutation primitives in `DirectoryEngine` without drifting into namespace policy, inode publication, allocation ownership, or sync ordering.

## Architectural Unit Context

- Functional goal: write-side directory-entry mutation in `DirectoryEngine`
- Final architectural owner: `ExfatFs` internal `DirectoryEngine` methods, consumed later by `ExfatInode` namespace owners
- Expected landing form: `DirectoryEngine` write methods plus owner-private helpers in `directory.rs`
- Architect artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/00_architect.md`

## Required Resolution Questions

- Refine the architected `DirectoryEngine` write boundary into a creator-ready spec without reopening the owner question.
- State how validated `ExfatDentrySet` values and committed allocation results are consumed without re-owning fileset validation or allocation search.
- Specify which mutation primitives belong here now: slot discovery, record placement/removal, overwrite rules, tombstoning, and directory-side handling when a write cannot stay in place.
- Decide whether a dedicated async artifact is needed; if not, say why explicitly.
- Define checker-owned test obligations for slot reuse, in-place rewrite, directory growth using committed allocation results, and separation from namespace policy.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_CORE_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_ASYNC_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_KTEST_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILESET-04B/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/03_designer_ktest.md`

## Forbidden Files

- production code
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- creator, checker, advisor, and reviewer artifacts for any component

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/designer.md`
- Based-on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/00_architect.md`

## Semantic Prior Inputs

- `DirectoryEngine` remains the stable `ExfatFs`-owned directory service.
- `EXR-FILESET-04B` remains the validated file-record boundary.
- `EXR-ALLOC-27` remains the owner of allocation search, reservation, and committed allocation results.

## Integration Prior Inputs

- Consume the current `directory.rs` owner shape as the foundation for later write methods; do not redesign read-side directory streaming inside this designer pass.
- Keep later `EXR-NAMESPACE-29` inode methods, opened-inode publication, and sync ordering out of scope except as downstream consumers of the mutation primitives.
- Use the local Linux summary only as orientation for directory-update shape. It does not override the refactor boundary.

## Workflow Prior Inputs

- Command-free designer lane.
- This lane may overlap with the active `EXR-PGCACHE-26` creator lane because the write sets are disjoint.
- Produce the standard split artifact set: core, async, and ktest.
- Stay designer-only; do not drift into creator implementation details beyond creator-ready obligations.

## Quality Prior Inputs

- Use the designer-role quality slice from `$exfat-subagent-workflow`.
- Keep mutation primitives subordinate to `DirectoryEngine`.
- Reject drift into namespace policy, allocator ownership, or a standalone directory-write manager.

## Temporary Interfaces And Exit Plan

- Do not authorize a standalone directory-write manager, namespace helper service, or sync/writeback layer in this designer pass.
- If the spec needs a temporary staging record or location handle, it must explicitly name the later owner or removal condition.

## Helper Justification

- Allowed helper surfaces are owner-private helpers that:
  - discover or reserve slot ranges inside one directory,
  - place or remove serialized validated file-record bytes,
  - and consume committed allocation results when directory growth is already decided.
- They must remain subordinate to `DirectoryEngine`.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with:
  - `EXR-PGCACHE-26` creator
  - later artifact-only planning for `EXR-NAMESPACE-29`

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/01_designer_core.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/02_designer_async.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/03_designer_ktest.md`

## Escalation Rule

- If the architect artifact is still too coarse to specify a stable directory-write boundary without deciding namespace policy, allocator ownership, or sync ordering, report the exact missing handshake and stop instead of guessing.
