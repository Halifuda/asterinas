<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-NAMESPACE-29-20260412-2126-ARCH`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-NAMESPACE-29/20260412-2126-architect-packet.md`
- Supersedes: None
- Role: `architect`
- Component: `EXR-NAMESPACE-29`
- Phase: `architect`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 21:26 CST`

## Goal

- Produce the architect artifact for `EXR-NAMESPACE-29`: the owner-first boundary for `ExfatInode` namespace mutation methods (`create`, `unlink`, `mkdir`, `rmdir`, and `rename`) that consume accepted read-side directory ownership plus the newly specified `DirectoryEngine` write primitives without absorbing allocator ownership or sync ordering.

## Architectural Unit Context

- Functional goal: `ExfatInode`-owned namespace mutation
- Final architectural owner: `ExfatInode`
- Expected landing form: owner methods on `inode.rs`
- Parent units:
  - accepted `EXR-DIR-OPS-23`
  - specified `EXR-DENTRY-WRITE-28`
  - accepted `EXR-UPCASE-20`
- Interfaces served:
  - later create/unlink/mkdir/rmdir/rename VFS methods
  - later consumers of namespace-visible directory mutation

## Required Resolution Questions

- What is the stable `ExfatInode` namespace-mutation boundary now that `DirectoryEngine` write primitives are specified?
- Which responsibilities stay on the inode owner versus on `DirectoryEngine`, the allocator, and later sync/write rows?
- How should `create`, `unlink`, `mkdir`, `rmdir`, and `rename` share one namespace owner without inventing a standalone namespace manager?
- Which upstream inputs are architecturally real prerequisites: validated record construction, upcase/name folding, directory-write primitives, committed allocation results, opened-inode publication, and later sync ownership?
- What initial work slices are safe once the owner boundary is fixed, and where should future collisions be expected?

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-RESET/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/inode.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/00_architect.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- production code
- designer, creator, checker, advisor, and reviewer artifacts for any component

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/architect.md`

## Semantic Prior Inputs

- `ExfatInode` remains the namespace-visible owner.
- `EXR-DENTRY-WRITE-28` remains the owner of directory-entry slot placement, overwrite, tombstoning, and growth consumption once a committed allocation result already exists.
- `EXR-ALLOC-27` remains the owner of allocation search, reservation intent, and commit. Namespace mutation may consume committed results but must not absorb allocator ownership.

## Integration Prior Inputs

- Use accepted directory-read ownership from `EXR-DIR-OPS-23` and accepted upcase/name-folding ownership from `EXR-UPCASE-20` as upstream prerequisites.
- The local Linux summary and legacy `exfat/inode.rs` are orientation aids only. They do not override the owner-first refactor boundary.
- Keep later sync/flush ordering for `EXR-SYNC-31` out of scope except as an explicit downstream dependency.

## Workflow Prior Inputs

- Command-free architect lane.
- This lane may overlap with the active `EXR-PGCACHE-26` checker and `EXR-ALLOC-27` creator because the write set is artifact-only and disjoint.
- Stay architect-only; do not drift into designer details or production edit plans beyond boundary-safe slice recommendations.

## Quality Prior Inputs

- Use architect-level boundary guidance only.
- Reject any split that turns namespace mutation into a standalone manager instead of `ExfatInode` owner methods.

## Temporary Interfaces And Exit Plan

- No production temporary interface is authorized in this architect pass unless the artifact names the future owner or removal condition.
- Do not authorize a namespace service layer, directory-write manager, or sync/writeback coordinator here.

## Helper Justification

- Any helper-like surface proposed here must remain subordinate to `ExfatInode` namespace ownership and justified by stable owner boundaries rather than packet convenience.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with:
  - `EXR-PGCACHE-26` checker
  - `EXR-ALLOC-27` creator

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/00_architect.md`

## Escalation Rule

- If the namespace boundary cannot be defined without absorbing allocator ownership, directory-write ownership, or sync ordering, report the exact missing dependency and stop instead of inventing a staging owner.
