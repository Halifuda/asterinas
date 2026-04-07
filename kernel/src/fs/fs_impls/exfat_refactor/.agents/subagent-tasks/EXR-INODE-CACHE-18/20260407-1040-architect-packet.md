<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-INODE-CACHE-18-20260407-1040-ARCH`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CACHE-18/20260407-1040-architect-packet.md`
- Supersedes: None
- Role: `architect`
- Component: `EXR-INODE-CACHE-18`
- Phase: `architect`
- Authorizing main agent: `main-agent`
- Date: `2026-04-07 10:40 CST`

## Goal

- Produce the architect artifact for `EXR-INODE-CACHE-18`: the owner-first boundary for the `ExfatFs`-owned opened-inode table and validated `InodeKey`, using the accepted `EXR-FS-CORE-16` and `EXR-INODE-CORE-17` architect artifacts as hard inputs.

## Architectural Unit Context

- Functional goal: define opened-inode identity and cache ownership under `ExfatFs`, including the root special case, without turning `InodeKey` into a standalone helper-only component.
- Final architectural owner: `ExfatFs`.
- Expected landing form: owner-internal state plus validated `InodeKey`.
- Parent unit: none. This is a tracked functional unit in the live board.
- Interfaces served: `EXR-FS-OPEN-22`, `EXR-DIR-OPS-23`, namespace/read/write consumers that need stable inode identity.

## Required Resolution Questions

- What exact inode identity facts form `InodeKey`, and which are derived from `ExfatDentrySet` / directory location rather than generated ad hoc?
- Where does the opened-inode table live inside `ExfatFs`, and how does it hand out `Arc<ExfatInode>` without creating a filesystem/inode ownership cycle?
- How does the root special case fit without using root as a fake cache key?
- Which work slices are safe and which must wait for `EXR-FS-OPEN-22`?
- What lock/order or serialization obligations must designer work make explicit?

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-RESET/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/inode.rs`
- Read-only inspection commands inside `/home/halifuda/asterinas` are allowed.

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/00_architect.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Production code under `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/`
- Other role artifacts outside the write set.

## Required Inputs

- Role-scoped protocol files: `COMMON_SUBAGENT.md`, `ARCHITECT.md`.
- Accepted architect inputs: `EXR-FS-CORE-16/00_architect.md`, `EXR-INODE-CORE-17/00_architect.md`.

## Semantic Prior Inputs

- Use `linux-exFAT-implementation-summary.md` topic “Inode hashing and opened-inode identity” for orientation only.
- No direct Linux source reads are authorized. If exact Linux hash behavior is needed, stop and report.
- Use accepted dentry-set and chain boundaries as semantic inputs.

## Integration Prior Inputs

- Use `ASTERINAS_ARCHITECT_PRIORS.md` profile `I-ARCH`, especially `ExfatFs`, `ExfatInode`, and VFS inode contract context.
- Legacy exFAT fs/inode code is integration context only.

## Workflow Prior Inputs

- Command-free architect lane.
- May overlap with designer lanes and `EXR-DIR-ENGINE-19` architect lane because write sets are disjoint.
- Workflow priors may shape work slices only after ownership and identity semantics are resolved.

## Quality Prior Inputs

- Use `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-ARCH`.
- Focus on not recreating the rolled-back standalone `INOKEY` drift.

## Temporary Interfaces And Exit Plan

- No production temporary interface is authorized in this architect pass, but the artifact may recommend one only if it names the future owner/removal condition.

## Helper Justification

- Any `InodeKey` helper must be justified as part of `ExfatFs` opened-inode table ownership, not as a free helper.

## Allowed Commands

- Read-only shell commands only.
- No build/test/QEMU/format commands.

## Parallelism Classification

- Lane class: command-free.
- May overlap with: `EXR-DIR-ENGINE-19` architect and Wave A designer lanes.
- Known conflicts: none beyond forbidden write sets.

## Execution Environment

- Host read-only inspection from `/home/halifuda/asterinas`.

## Execution Lock

- No execution lock is needed.

## Stop Condition

- Stop after writing `components/EXR-INODE-CACHE-18/00_architect.md`.

## Escalation Rule

- If the accepted `ExfatFs`/`ExfatInode` architect artifacts are insufficient to define cache ownership, stop and report the missing handshake requirement instead of inventing a standalone key module.
