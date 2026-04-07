<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-INODE-CORE-17-20260407-1021-ARCH`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CORE-17/20260407-1021-architect-packet.md`
- Supersedes: None
- Role: `architect`
- Component: `EXR-INODE-CORE-17`
- Phase: `architect`
- Authorizing main agent: `main-agent`
- Date: `2026-04-07 10:21 CST`

## Goal

- Produce the architect artifact for `EXR-INODE-CORE-17`: the owner-first boundary for introducing `ExfatInode` as the stable VFS `Inode` trait carrier, while keeping inode cache, directory operations, file mapping, page cache, read/write, and namespace mutation out of this unit unless explicitly justified as a temporary seam.

## Architectural Unit Context

- Functional goal: define the minimal inode owner that can represent exFAT inode identity and metadata state for later VFS behavior without recreating the failed standalone metadata-shell drift.
- Final architectural owner: `ExfatInode`.
- Expected landing form: trait-carrier type plus owner state; no standalone metadata-shell component.
- Parent unit: none. This is a tracked functional unit in the live board.
- Interfaces ultimately served:
  - VFS `Inode` and `InodeIo`;
  - future `EXR-INODE-CACHE-18` opened-inode table;
  - future `EXR-DIR-OPS-23`, `EXR-FILE-MAP-24`, `EXR-READ-OPS-25`, `EXR-PGCACHE-26`, `EXR-NAMESPACE-29`, and `EXR-WRITE-30`;
  - sibling `EXR-FS-CORE-16` `FileSystem::root_inode()` contract.

## Required Resolution Questions

- Unit-definition questions:
  - What is the smallest coherent `EXR-INODE-CORE-17` unit that introduces `ExfatInode` without absorbing inode cache, lookup, readdir, file mapping, page-cache backend, read/write, or namespace mutation?
  - Which VFS `Inode` / `InodeIo` methods can have meaningful initial behavior now, and which should be explicit temporary seams or deferred defaults?
- Owner-definition questions:
  - Which metadata fields belong in `ExfatInode` now, and which belong to later units?
  - How should `ExfatInode` reference its owning `ExfatFs` without forcing `EXR-INODE-CACHE-18` into this unit?
  - Which identity information from `ExfatDentrySet` and `ExfatChain` is needed now, and which belongs to the later `InodeKey` cache unit?
- Work-slice and parallel-wave questions:
  - What handshake assumptions must be shared with `EXR-FS-CORE-16`, especially for root inode exposure?
  - Which candidate creator slices are safe for `EXR-INODE-CORE-17`, and which ones must wait for `EXR-FS-CORE-16` or `EXR-INODE-CACHE-18`?
  - Which likely file landing zones avoid collisions with the parallel `EXR-FS-CORE-16` architect lane?

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
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FS-CORE-16/20260407-1021-architect-packet.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/file_system.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode_ext.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/inode.rs`
- Read-only inspection commands inside `/home/halifuda/asterinas` are allowed when needed to understand current boundaries.

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/00_architect.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Production code under `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/`
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
  - read the `EXR-FS-CORE-16` architect packet and record handshake assumptions, but do not wait for that artifact or edit it.

## Semantic Prior Inputs

- Use accepted prior-derived behavior from:
  - `EXR-FILESET-04B` through `fileset.rs`;
  - `EXR-CHAIN-03B` through `fat.rs`;
  - `linux-exFAT-implementation-summary.md` only as high-level orientation for inode ownership and identity.
- No direct Linux source reads are authorized for this packet by default. If exact Linux inode hashing or writeback behavior appears necessary, stop and report the missing input rather than widening scope.
- Precedence:
  - accepted Microsoft-derived dentry-set and chain artifacts for already-validated value boundaries;
  - Linux summary only for implementation orientation where this unit needs high-level ownership context.

## Integration Prior Inputs

- Use `ASTERINAS_ARCHITECT_PRIORS.md` profile `I-ARCH`, especially the Inode contract, PageCache contract as future context, current `ExfatInode` runtime object, and runtime ownership sections.
- Use exact local VFS surfaces:
  - `kernel/src/fs/vfs/fs_apis/inode.rs`
  - `kernel/src/fs/vfs/fs_apis/inode_ext.rs`
  - `kernel/src/fs/vfs/fs_apis/file_system.rs`
- Legacy `kernel/src/fs/fs_impls/exfat/fs.rs` and `inode.rs` are integration context, not semantic targets.

## Workflow Prior Inputs

- This is a command-free architect lane.
- This lane may overlap with the `EXR-FS-CORE-16` architect lane because write sets are disjoint.
- Workflow priors may shape candidate work slices after owner and trait-carrier boundaries are decided.
- Do not invent an inode metadata-shell component or fold `EXR-INODE-CACHE-18` into this unit just to make `EXR-FS-CORE-16` easier.

## Quality Prior Inputs

- Use `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-ARCH`.
- In scope: owner clarity, boundary clarity, explicit temporary seam exit plans, file landing zones, and realistic creator slice sizing.
- Out of scope: creator-local naming or formatting decisions.

## Prior Delivery Notes

- This packet is narrow: it asks only for the inode trait-carrier boundary, not inode cache, directory operations, file mapping, page-cache backend, read/write, allocation, namespace mutation, or sync ordering.
- The packet intentionally includes the sibling `EXR-FS-CORE-16` packet so the architect can name cross-lane assumptions without rewriting the sibling unit.

## Temporary Interfaces And Exit Plan

- Temporary construction seams are allowed only if the architect artifact names:
  - why the seam is needed in `EXR-INODE-CORE-17`,
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
- May overlap with: `EXR-FS-CORE-16` architect lane and other command-free lanes with disjoint write sets.
- Known conflicts: production code writes and scheduler-owned board edits are forbidden.

## Execution Environment

- Host or Docker: host-side read-only inspection is sufficient.
- Required command prefix: none.
- Required working directory: `/home/halifuda/asterinas`.
- Isolation notes: command-free only; do not run compile or runtime commands.

## Execution Lock

- No execution lock is needed because this task is command-free.

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/00_architect.md`.
- Do not edit `COMPONENT_INDEX.md`, production code, or the sibling `EXR-FS-CORE-16` artifact.

## Escalation Rule

- If the `ExfatFs` owner contract is too unsettled to architect this unit, record the exact missing handshake requirement and recommend paired reconciliation instead of silently creating an inode cache, fake filesystem owner, or standalone metadata-shell boundary.
