<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-INODE-CORE-17-20260407-1043-CREATE-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CORE-17/20260407-1043-creator-serial-packet.md`
- Supersedes: None
- Role: `creator`
- Component: `EXR-INODE-CORE-17`
- Phase: `serial implementation`
- Authorizing main agent: `main-agent`
- Date: `2026-04-07 10:43 CST`

## Goal

- Implement the narrow `ExfatInode` carrier and metadata owner from the accepted designer spec, without inode cache, page cache, directory ops, read/write, namespace mutation, or sync behavior.

## Architectural Unit Context

- Functional goal: introduce `ExfatInode` as the stable VFS inode carrier with owner-private metadata and a weak `ExfatFs` back-reference.
- Final architectural owner: `ExfatInode`.
- Expected landing form: trait-carrier type plus owner state.
- Parent unit: none.
- Interfaces served: VFS `Inode` / `InodeIo`, sibling `EXR-FS-CORE-16`, future inode-cache and read/write units.

## Required Resolution Questions

- Follow the accepted designer artifacts. Do not redesign the unit.
- Implement metadata accessors and explicit temporary seams.
- Do not edit `mod.rs`; the `EXR-FS-CORE-16` creator owns shared module declarations.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CREATOR_LOG_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/file_system.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode_ext.rs`
- Read-only inspection commands inside `/home/halifuda/asterinas` are allowed.

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/10_creator_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Sibling `EXR-FS-CORE-16` artifacts.

## Required Inputs

- Role-scoped protocol files: `COMMON_SUBAGENT.md`, `CREATOR.md`.
- Designer spec: `EXR-INODE-CORE-17/01_designer_core.md` and `03_designer_ktest.md`.
- Creator log template.

## Semantic Prior Inputs

- Use accepted `ExfatDentrySet` and `ExfatChain` code plus designer-derived constraints only. Do not reopen Linux behavior.

## Integration Prior Inputs

- Use exact VFS `Inode`, `InodeIo`, `FileSystem`, and `InodeExt` surfaces.
- Use sibling `ExfatFs` interface assumptions from the FS designer artifact.

## Workflow Prior Inputs

- Command-free creator lane. Do not run compile commands.
- May overlap with `EXR-FS-CORE-16` creator because this packet does not edit `mod.rs` or `fs.rs`.

## Quality Prior Inputs

- Use `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-CREATE`.
- Keep constructor/helper surfaces crate-private unless a spec-named caller exists.

## Temporary Interfaces And Exit Plan

- Required data-path seam comment:
  - `// Temporary seam: EXR-READ-OPS-25, EXR-WRITE-30, and EXR-PGCACHE-26 will own this path.`
- `resize()`, `set_mode()`, `set_owner()`, and `set_group()` must reject or remain explicit temporary seams; do not mutate hidden writeback state.

## Helper Justification

- No `InodeKey` helper or opened-inode table helper is authorized in this unit.
- Do not expose raw field getters without a named future caller in the designer spec.

## Allowed Commands

- Read-only shell commands only.
- No build, test, format, or QEMU commands.

## Parallelism Classification

- Lane class: command-free.
- May overlap with: `EXR-FS-CORE-16` creator lane.
- Known conflicts: `mod.rs` is forbidden here and owned by the FS creator lane.

## Execution Environment

- Host read-only inspection and file edits under `/home/halifuda/asterinas`.

## Execution Lock

- No execution lock is needed.

## Stop Condition

- Stop after implementing the assigned pass and writing `EXR-INODE-CORE-17/10_creator_serial.md`. Do not run checker work.

## Escalation Rule

- If metadata ownership cannot be implemented without cache or data-path behavior, report the gap instead of widening into later units.
