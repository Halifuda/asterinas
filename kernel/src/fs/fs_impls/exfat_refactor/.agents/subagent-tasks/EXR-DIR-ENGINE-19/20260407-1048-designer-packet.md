<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DIR-ENGINE-19-20260407-1048-DESIGN`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-ENGINE-19/20260407-1048-designer-packet.md`
- Supersedes: None
- Role: `designer`
- Component: `EXR-DIR-ENGINE-19`
- Phase: `designer`
- Authorizing main agent: `main-agent`
- Date: `2026-04-07 10:48 CST`

## Goal

- Produce the designer artifact set for `EXR-DIR-ENGINE-19`, specifying the read-only `DirectoryEngine` record-stream service without absorbing upcase policy, bitmap policy, VFS directory ops, or write-side mutation.

## Architectural Unit Context

- Functional goal: define `ExfatFs`-owned internal directory record streaming over `ExfatChain` into validated `ExfatDentrySet` records.
- Final architectural owner: `ExfatFs`.
- Expected landing form: owner-internal service `DirectoryEngine`.
- Interfaces served: future `EXR-UPCASE-20`, `EXR-BITMAP-21`, `EXR-FS-OPEN-22`, `EXR-DIR-OPS-23`, and later `EXR-DENTRY-WRITE-28`.

## Required Resolution Questions

- Point to the accepted architect artifact for the boundary.
- Specify the read-only scan state, record-stream contract, and system-entry candidate output without name policy or bitmap interpretation.
- Specify whether `02_designer_async.md` is needed; if omitted, record why in `01_designer_core.md`.
- State checker-owned ktest obligations for directory stream boundaries.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- Read-only inspection commands inside `/home/halifuda/asterinas` are allowed.

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/03_designer_ktest.md`

## Forbidden Files

- Production code, `COMPONENT_INDEX.md`, main-agent handoffs, and other components' artifacts.

## Required Inputs

- Role-scoped protocol files: `COMMON_SUBAGENT.md`, `DESIGNER.md`.
- Accepted architect artifact: `EXR-DIR-ENGINE-19/00_architect.md`.

## Semantic Prior Inputs

- Use accepted `EXR-IO-02`, `EXR-CHAIN-03B`, `EXR-DENTRY-04A`, and `EXR-FILESET-04B` code. Do not read Linux source.

## Integration Prior Inputs

- Use `ASTERINAS_ARCHITECT_PRIORS.md` profile `I-DESIGN`.

## Workflow Prior Inputs

- Command-free designer lane. May overlap with `EXR-INODE-CACHE-18` designer and Wave A creator lanes because write sets are disjoint.

## Quality Prior Inputs

- Use `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-DESIGN`.

## Temporary Interfaces And Exit Plan

- No temporary production surface is expected unless explicitly justified with an exit plan.

## Helper Justification

- Any helper surface must be justified by the `DirectoryEngine` owner and named future consumer.

## Allowed Commands

- Read-only shell commands only. No build/test/QEMU/format commands.

## Parallelism Classification

- Lane class: command-free.
- May overlap with: `EXR-INODE-CACHE-18` designer and Wave A creator lanes.

## Execution Environment

- Host read-only inspection from `/home/halifuda/asterinas`.

## Execution Lock

- No execution lock is needed.

## Stop Condition

- Stop after writing the assigned designer artifact set. If `02_designer_async.md` is omitted, explicitly record why in `01_designer_core.md`.

## Escalation Rule

- If the read-only engine cannot be specified without upcase/bitmap/VFS dir ops, report the gap instead of widening into later units.
