<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-FS-CORE-16-20260407-1043-CREATE-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FS-CORE-16/20260407-1043-creator-serial-packet.md`
- Supersedes: None
- Role: `creator`
- Component: `EXR-FS-CORE-16`
- Phase: `serial implementation`
- Authorizing main agent: `main-agent`
- Date: `2026-04-07 10:43 CST`

## Goal

- Implement the narrow `ExfatFs` owner skeleton from the accepted designer spec. This pass also owns the shared `mod.rs` declaration edit for both Wave A production modules to avoid a sibling write conflict.

## Architectural Unit Context

- Functional goal: introduce `ExfatFs` as the stable VFS `FileSystem` carrier and runtime-state root.
- Final architectural owner: `ExfatFs`.
- Expected landing form: trait-carrier type plus owner state.
- Parent unit: none.
- Interfaces served: VFS `FileSystem`, future `EXR-FS-OPEN-22`, future `EXR-SYNC-31`, sibling `EXR-INODE-CORE-17`.

## Required Resolution Questions

- Follow the accepted designer artifacts. Do not redesign the unit.
- Implement `name()`, `sb()`, `fs_event_subscriber_stats()`, explicit `root_inode()` temporary seam, and placeholder `sync()`.
- Own the `mod.rs` declaration edit for `fs` and `inode` modules so the sibling creator must not touch `mod.rs`.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CREATOR_LOG_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/file_system.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- Read-only inspection commands inside `/home/halifuda/asterinas` are allowed.

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/10_creator_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Sibling `EXR-INODE-CORE-17` artifacts.

## Required Inputs

- Role-scoped protocol files: `COMMON_SUBAGENT.md`, `CREATOR.md`.
- Designer spec: `EXR-FS-CORE-16/01_designer_core.md` and `03_designer_ktest.md`.
- Creator log template.

## Semantic Prior Inputs

- Use accepted superblock code and designer-derived constraints only. Do not reopen Linux behavior.

## Integration Prior Inputs

- Use exact VFS `FileSystem` and `Inode` trait surfaces.
- Use `ExfatSuperBlock` as the normalized geometry input.

## Workflow Prior Inputs

- Command-free creator lane. Do not run compile commands.
- May overlap with `EXR-INODE-CORE-17` creator because this packet owns `mod.rs`; sibling must write only `inode.rs`.

## Quality Prior Inputs

- Use `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-CREATE`.
- Keep visibility narrow and avoid helper wrappers without designer-backed justification.

## Temporary Interfaces And Exit Plan

- Required `root_inode()` seam comment:
  - `// Temporary seam: EXR-FS-OPEN-22 will install the real root inode after EXR-INODE-CORE-17 lands.`
- `sync()` remains a placeholder and must not begin real flush ordering.

## Helper Justification

- Do not add helper APIs beyond what is needed to construct/test the owner skeleton. Field-exposing accessors require a named caller from the spec.

## Allowed Commands

- Read-only shell commands only.
- No build, test, format, or QEMU commands.

## Parallelism Classification

- Lane class: command-free.
- May overlap with: `EXR-INODE-CORE-17` creator lane because write sets are disjoint.
- Known conflicts: this packet owns `mod.rs`; sibling creator must not edit it.

## Execution Environment

- Host read-only inspection and file edits under `/home/halifuda/asterinas`.

## Execution Lock

- No execution lock is needed.

## Stop Condition

- Stop after implementing the assigned pass and writing `EXR-FS-CORE-16/10_creator_serial.md`. Do not run checker work.

## Escalation Rule

- If implementing `root_inode()` requires a real inode or mount/open path, leave the explicit temporary seam and report the limitation instead of widening scope.
