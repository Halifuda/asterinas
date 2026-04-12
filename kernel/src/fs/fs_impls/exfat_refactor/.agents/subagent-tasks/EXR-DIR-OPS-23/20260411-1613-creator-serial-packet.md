<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DIR-OPS-23-20260411-1613-CREATE-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-OPS-23/20260411-1613-creator-serial-packet.md`
- Supersedes: None
- Role: `creator`
- Component: `EXR-DIR-OPS-23`
- Phase: `serial creator`
- Authorizing main agent: `main-agent`
- Date: `2026-04-11 16:13 CST`

## Goal

- Implement the read-only `ExfatInode` directory surface in `inode.rs` by landing `lookup` and `readdir_at` as inode-owned methods that consume `DirectoryEngine`, filesystem-owned canonicalization, and filesystem-owned opened-inode reuse without widening into mount/open, mutation, or file-data behavior.

## Architectural Unit Context

- Functional goal: `ExfatInode` read-only directory operations
- Final architectural owner: `ExfatInode`
- Expected landing form: owner methods in `inode.rs`
- Interfaces served:
  - VFS `Inode::lookup`
  - VFS `Inode::readdir_at`
  - published-root and child reuse paths already owned by `ExfatFs`

## Required Resolution Questions

- Implement directory-only `lookup` and `readdir_at` on `ExfatInode` according to the accepted designer set.
- Preserve non-directory rejection behavior outside directory inodes.
- Use `DirectoryEngine` as the record-stream source, `UpcaseTable` as the canonicalization source, and `ExfatFs` as the child-handle reuse boundary.
- Keep `readdir_at` continuation stable without introducing a long-lived scanner owner.
- Do not widen into mount/open sequencing, namespace mutation, allocator policy, file mapping, or data I/O.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/utils/dirent_visitor.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/inode.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/10_creator_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- checker, reviewer, advisor, and handoff artifacts

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/creator.md`
- Implement against the accepted `EXR-DIR-OPS-23` designer artifacts only; do not reopen architect or designer decisions unless the packet's escalation rule triggers.

## Semantic Prior Inputs

- Use the accepted `EXR-DIR-OPS-23` designer set as authoritative for behavior.
- The legacy exFAT source path in the read set is reference material for directory-op details only; it does not override the refactor's accepted owner boundary or helper constraints.
- Name-sensitive matching must consume filesystem-owned canonicalization instead of local ad hoc folding or hashing.

## Integration Prior Inputs

- `EXR-FS-OPEN-22` already owns root publication and mount/open sequencing; this row begins after a directory inode already exists.
- `DirectoryEngine` remains an `ExfatFs`-owned record stream and must be consumed read-only.
- Child-handle reuse remains filesystem-owned through `ExfatFs`; `ExfatInode` must not grow a second cache or publication owner.
- `EXR-NAMESPACE-29` owns later mutation, so this pass must not add create, unlink, mkdir, rmdir, or rename behavior.

## Workflow Prior Inputs

- Command-free creator lane.
- This is the loop's only production creator round.
- Keep the implementation inside `inode.rs`; if the accepted designer behavior cannot land cleanly there, stop and report the missing handshake instead of widening into `fs.rs` or `directory.rs`.
- Do not run compile or test commands; checker will own executable verification.

## Quality Prior Inputs

- Use the creator-role quality slice from `$exfat-subagent-workflow`.
- Keep any helper surface owner-private to `ExfatInode`.
- Prefer owner methods or owner-private associated helpers over module-scope free functions.
- If you add a private helper, local type, accessor, or temporary seam, record it in the creator artifact together with its final owner or removal condition.

## Temporary Interfaces And Exit Plan

- Do not introduce a separate lookup service, scanner owner, or mutation shell.
- The existing unsupported `read_at` and `write_at` seams in `inode.rs` remain outside this component and should not be altered except where directly required by the assigned directory-op methods.

## Helper Justification

- Allowed owner-private helpers may:
  - derive a directory scan input from the current inode,
  - compare a candidate record name against the caller name using filesystem-owned canonicalization,
  - materialize or reuse a child inode through `ExfatFs`,
  - and project visible file records into `readdir_at` dirents.
- Do not add field-exposing accessors or convenience helpers unless they directly serve one of the above in this pass.
- Do not let a helper become a floating module-level convenience seam or a hidden second owner for scanning or canonicalization.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free production edit`
- May overlap with:
  - artifact-only planning lanes with disjoint write sets
- Known conflicts:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - later checker or reviewer lanes for `EXR-DIR-OPS-23`

## Execution Environment

- Host workspace only
- This task is command-free. Do not add compile or runtime commands on your own.

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/10_creator_serial.md`.
- Do not proceed into checker work.

## Escalation Rule

- If implementing the accepted designer behavior appears to require edits outside `inode.rs`, or if the accepted designer set is insufficient to choose a stable `readdir_at` continuation strategy, report the exact missing handshake and stop instead of widening scope.
