<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-FS-CORE-16-20260407-1021-ARCH`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FS-CORE-16/20260407-1021-architect-packet.md`
- Supersedes: None
- Role: `architect`
- Component: `EXR-FS-CORE-16`
- Phase: `architect`
- Authorizing main agent: `main-agent`
- Date: `2026-04-07 10:21 CST`

## Goal

- Produce the architect artifact for `EXR-FS-CORE-16`: the owner-first boundary for introducing `ExfatFs` as the stable VFS `FileSystem` trait carrier and runtime-state root for the refactored exFAT implementation.

## Architectural Unit Context

- Functional goal: define the minimal filesystem-wide owner that later mount/open, inode cache, directory services, bitmap/upcase state, allocation, and sync behavior can attach to without creating ownerless staging surfaces.
- Final architectural owner: `ExfatFs`.
- Expected landing form: trait-carrier type plus owner state; possibly a temporary construction seam only if it has a named exit plan.
- Parent unit: none. This is a tracked functional unit in the live board.
- Interfaces ultimately served:
  - VFS `FileSystem`;
  - future `ExfatFs::open(...)` in `EXR-FS-OPEN-22`;
  - future `EXR-INODE-CORE-17` root/inode contract;
  - future `EXR-INODE-CACHE-18`, `EXR-DIR-ENGINE-19`, `EXR-UPCASE-20`, `EXR-BITMAP-21`, and `EXR-SYNC-31`.

## Required Resolution Questions

- Unit-definition questions:
  - What is the smallest coherent `EXR-FS-CORE-16` unit that introduces `ExfatFs` without absorbing mount sequencing from `EXR-FS-OPEN-22`?
  - Which `FileSystem` trait methods can be part of this unit now, and which must be explicitly deferred?
- Owner-definition questions:
  - Which fields or owner-state placeholders belong in `ExfatFs` now, and which belong to later units?
  - How should `FileSystem::root_inode() -> Arc<dyn Inode>` be handled while `EXR-INODE-CORE-17` is a parallel sibling and not yet implemented?
  - Is `todo!` or `unimplemented!` acceptable for any temporary seam in this unit? If yes, name the exact seam, why it is unreachable from the registered legacy filesystem, and which later component removes it.
- Work-slice and parallel-wave questions:
  - What candidate creator slices are safe for `EXR-FS-CORE-16`, and which ones must wait for `EXR-INODE-CORE-17`?
  - Which likely file landing zones avoid collisions with the parallel `EXR-INODE-CORE-17` architect lane?
  - What assumptions should the `EXR-INODE-CORE-17` architect consume as handshake requirements rather than as settled code?

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-RESET/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/river-anvil-20260407-1010-resume-board-validation.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/TASK_PACKET_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CORE-17/20260407-1021-architect-packet.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/file_system.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/inode.rs`
- Read-only inspection commands inside `/home/halifuda/asterinas` are allowed when needed to understand current boundaries.

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/00_architect.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Production code under `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/`
- Designer, creator, checker, advisor, or reviewer artifacts

## Required Inputs

- Role-scoped protocol files accompanying this packet:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- Required template:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- Required board and reset context:
  - `COMPONENT_INDEX.md`
  - `WORKSPACE-ARCH-RESET/00_architect.md`
  - `river-anvil-20260407-1010-resume-board-validation.md`
- Parallel sibling context:
  - read the `EXR-INODE-CORE-17` architect packet and record handshake assumptions, but do not wait for that artifact or edit it.

## Semantic Prior Inputs

- Use accepted prior-derived behavior from:
  - `EXR-BOOT-01` / `EXR-SBGEOM-15` through `boot_sector.rs` and `super_block.rs`;
  - `linux-exFAT-implementation-summary.md` only as high-level orientation for mount/bootstrap ownership.
- No direct Linux source reads are authorized for this packet by default. If exact Linux mount sequencing appears necessary, stop and report the missing input rather than widening scope.
- Precedence:
  - accepted Microsoft-derived boot/superblock artifacts for already-validated geometry;
  - Linux summary only for implementation orientation where the current unit needs high-level ownership context.

## Integration Prior Inputs

- Use `ASTERINAS_ARCHITECT_PRIORS.md` profile `I-ARCH`, especially the FileSystem contract, current `ExfatFs` runtime object, and mount/open sequence sections.
- Use exact local VFS surfaces:
  - `kernel/src/fs/vfs/fs_apis/file_system.rs`
  - `kernel/src/fs/vfs/fs_apis/inode.rs`
- Legacy `kernel/src/fs/fs_impls/exfat/fs.rs` and `inode.rs` are integration context, not semantic targets.

## Workflow Prior Inputs

- This is a command-free architect lane.
- This lane may overlap with the `EXR-INODE-CORE-17` architect lane because write sets are disjoint.
- Workflow priors may shape candidate work slices after owner and trait-carrier boundaries are decided.
- Do not invent a helper-only component to make the `root_inode()` dependency easier.

## Quality Prior Inputs

- Use `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-ARCH`.
- In scope: owner clarity, boundary clarity, explicit temporary seam exit plans, file landing zones, and realistic creator slice sizing.
- Out of scope: creator-local naming or formatting decisions.

## Prior Delivery Notes

- This packet is narrow: it asks only for the filesystem-wide trait carrier boundary, not mount/open sequencing, inode cache details, directory services, upcase/bitmap loading, allocation, read/write, or sync ordering.
- The packet intentionally includes the sibling `EXR-INODE-CORE-17` packet so the architect can name cross-lane assumptions without rewriting the sibling unit.

## Temporary Interfaces And Exit Plan

- Temporary construction seams are allowed only if the architect artifact names:
  - why the seam is needed in `EXR-FS-CORE-16`,
  - why it is unreachable from the registered legacy filesystem,
  - the later component that removes or absorbs it,
  - and the exact short code comment future creator work must use.

## Helper Justification

- No new production helper is authorized by this architect packet.
- If the artifact recommends short helpers or accessors, it must name the expected cross-module caller or trust boundary and explain why the helper should exist now.

## Allowed Commands

- Read-only shell commands for inspection inside `/home/halifuda/asterinas`.
- No build, test, QEMU, formatting, or production-modifying commands.

## Parallelism Classification

- Lane class: command-free.
- May overlap with: `EXR-INODE-CORE-17` architect lane and other command-free lanes with disjoint write sets.
- Known conflicts: production code writes and scheduler-owned board edits are forbidden.

## Execution Environment

- Host or Docker: host-side read-only inspection is sufficient.
- Required command prefix: none.
- Required working directory: `/home/halifuda/asterinas`.
- Isolation notes: command-free only; do not run compile or runtime commands.

## Execution Lock

- No execution lock is needed because this task is command-free.

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/00_architect.md`.
- Do not edit `COMPONENT_INDEX.md`, production code, or the sibling `EXR-INODE-CORE-17` artifact.

## Escalation Rule

- If the `root_inode()` dependency makes the unit boundary invalid as currently planned, record the boundary problem and recommend a paired or re-sliced plan instead of silently creating a fake owner or widening into `EXR-INODE-CORE-17`.
