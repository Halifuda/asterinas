<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-INODE-CACHE-18-20260407-1048-DESIGN`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CACHE-18/20260407-1048-designer-packet.md`
- Supersedes: None
- Role: `designer`
- Component: `EXR-INODE-CACHE-18`
- Phase: `designer`
- Authorizing main agent: `main-agent`
- Date: `2026-04-07 10:48 CST`

## Goal

- Produce the designer artifact set for `EXR-INODE-CACHE-18`, specifying the `ExfatFs`-owned opened-inode table and validated `InodeKey` boundary without expanding into mount/open sequencing or directory traversal.

## Architectural Unit Context

- Functional goal: define opened-inode identity and cache ownership under `ExfatFs`, including the root special case.
- Final architectural owner: `ExfatFs`.
- Expected landing form: owner-internal state plus validated `InodeKey`.
- Interfaces served: future `EXR-FS-OPEN-22`, lookup/open reuse, and later VFS operations needing stable inode identity.

## Required Resolution Questions

- Point to the accepted architect artifact for the boundary.
- Specify `InodeKey` fields and validation rules without making it a free helper module.
- Specify opened-inode table operations, root special-case treatment, and lock/serialization obligations.
- Decide whether `02_designer_async.md` is needed; if omitted, record why in `01_designer_core.md`.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- Read-only inspection commands inside `/home/halifuda/asterinas` are allowed.

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/03_designer_ktest.md`

## Forbidden Files

- Production code, `COMPONENT_INDEX.md`, main-agent handoffs, and other components' artifacts.

## Required Inputs

- Role-scoped protocol files: `COMMON_SUBAGENT.md`, `DESIGNER.md`.
- Accepted architect artifact: `EXR-INODE-CACHE-18/00_architect.md`.

## Semantic Prior Inputs

- Use accepted architect artifact and accepted value-type boundaries only. Do not read Linux source.

## Integration Prior Inputs

- Use `ASTERINAS_ARCHITECT_PRIORS.md` profile `I-DESIGN`.

## Workflow Prior Inputs

- Command-free designer lane. May overlap with `EXR-DIR-ENGINE-19` designer and Wave A creator lanes because write sets are disjoint.

## Quality Prior Inputs

- Use `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-DESIGN`.

## Temporary Interfaces And Exit Plan

- No temporary production surface is expected unless explicitly justified with an exit plan.

## Helper Justification

- `InodeKey` helpers must be justified by `ExfatFs` opened-inode table ownership.

## Allowed Commands

- Read-only shell commands only. No build/test/QEMU/format commands.

## Parallelism Classification

- Lane class: command-free.
- May overlap with: `EXR-DIR-ENGINE-19` designer and Wave A creator lanes.

## Execution Environment

- Host read-only inspection from `/home/halifuda/asterinas`.

## Execution Lock

- No execution lock is needed.

## Stop Condition

- Stop after writing the assigned designer artifact set. If `02_designer_async.md` is omitted, explicitly record why in `01_designer_core.md`.

## Escalation Rule

- If the cache boundary cannot be specified without mount/open sequencing, report the gap instead of widening into `EXR-FS-OPEN-22`.
