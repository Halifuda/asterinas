<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-SYSROOT-06-ARCH-20260404-1402`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-SYSROOT-06/20260404-1402-architect-packet.md`
- Supersedes: none
- Role: architect
- Component: `EXR-SYSROOT-06`
- Phase: architect
- Authorizing main agent: main-agent
- Date: 2026-04-04 14:02 CST

## Goal

- Write the architect artifact for `EXR-SYSROOT-06` in `00_architect.md`. The component must cover only the root-directory scanner that discovers validated system entries needed by later loaders, with explicit boundaries that keep mount bootstrap, general directory iteration, upcase loading, bitmap loading, and inode/VFS behavior out of scope.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-05B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- `kernel/src/fs/fs_impls/exfat/dentry.rs`
- `kernel/src/fs/fs_impls/exfat/fs.rs`
- `kernel/src/fs/fs_impls/exfat/bitmap.rs`
- `kernel/src/fs/fs_impls/exfat/upcase_table.rs`
- `/home/halifuda/linux/fs/exfat/super.c`
- `/home/halifuda/linux/fs/exfat/balloc.c`
- `/home/halifuda/linux/fs/exfat/nls.c`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/00_architect.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- any file under `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/` other than `00_architect.md`
- all production Rust code

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-05B/00_architect.md` as a recent architect example
- Required code/reference inputs:
  - current refactor read-side modules listed in the read set
  - legacy Asterinas exFAT root bootstrap references in `fs.rs`, `bitmap.rs`, and `upcase_table.rs`
  - Linux source map inputs in `super.c`, `balloc.c`, and `nls.c`

## Semantic Prior Inputs

- Use:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
  - the listed Linux source files when the summary is too coarse
- Precedence:
  - Microsoft exFAT rules first for normative on-disk semantics
  - Linux exFAT implementation guidance second when the spec leaves design room
- Semantic focus:
  - root directory system-entry discovery
  - allocation bitmap and upcase-table entry identity at the root directory level
  - validation responsibilities that belong before later `UPCASE` and `BITMAP` loaders

## Local Architectural Prior Inputs

- Use selected slices from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`:
  - the source-map and component split sections
  - the notes that current mount code discovers bitmap and upcase from the root directory
  - the rule that mount/bootstrap, bitmap management, upcase loading, inode metadata, and directory walking remain distinct concerns
- Local architectural focus:
  - safe-Rust-only refactor boundary
  - explicit separation from mount-owned state and later shared-state objects
  - preserving dependency safety for `EXR-UPCASE-07A` and `EXR-BITMAP-08A`

## Quality Prior Inputs

- Use `Q-ARCH` and only boundary-level quality guidance from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - narrow component boundary
  - helper and temporary-surface discipline
  - exposing parallel-ready next waves
- Out of scope:
  - creator-local method naming or formatting detail
  - checker test planning detail

## Prior Delivery Notes

- This packet is intentionally narrow around one scheduler need: define the smallest useful system-entry scanner that makes `UPCASE-07A` and `BITMAP-08A` downstream-ready without folding them into one component.
- Use the Linux summary as an index, then read the named Linux source files only as needed for exact discovery/ownership behavior.
- Use the legacy Asterinas exFAT implementation as integration pressure only, not as semantic authority.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized in this architect pass.
- If the component appears to require a staging wrapper, stop and record that pressure explicitly instead of silently permitting it.

## Helper Justification

- No helper API is pre-authorized by this packet.
- If the architect believes a short helper is unavoidable, the artifact must name the downstream caller and explain why the helper belongs in `EXR-SYSROOT-06` rather than later `MOUNT`, `UPCASE`, `BITMAP`, or `DIR` work.

## Allowed Commands

- read-only shell inspection commands only

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - main-agent planning work
  - other command-free lanes with disjoint write sets
- Known conflicts:
  - any other lane attempting to write `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/00_architect.md`

## Execution Environment

- Host or Docker:
  - host workspace only
- Required command prefix:
  - read-only shell commands under `/home/halifuda/asterinas`
- Required working directory:
  - `/home/halifuda/asterinas`
- Isolation notes:
  - shared worktree; do not edit outside the write set
- This task is command-free and must not add compile or runtime commands.

## Execution Lock

- Lock script:
  - not applicable
- Lock path:
  - not applicable
- Lock metadata file:
  - not applicable
- This task does not include a command-producing checker stage.

## Stop Condition

- Stop after writing `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/00_architect.md`.
- Do not write designer artifacts, update the task board, or propose code edits beyond the architect artifact.

## Escalation Rule

- If the root scanner cannot be kept separate from mount bootstrap, general directory iteration, or the concrete `UPCASE` / `BITMAP` loaders, stop and report the blocking ambiguity in the architect artifact instead of widening the component.
