<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-20-20260407-1125-DESIGN`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-20/20260407-1125-designer-packet.md`
- Supersedes: None
- Role: `designer`
- Component: `EXR-UPCASE-20`
- Phase: `designer`
- Authorizing main agent: `main-agent`
- Date: `2026-04-07 11:25 CST`

## Goal

- Produce the designer artifact set for `EXR-UPCASE-20`, specifying the `ExfatFs`-owned validated `UpcaseTable` runtime state and owner-private folding/hash services without widening into directory scanning, VFS directory ops, namespace mutation, or mount/open sequencing.

## Architectural Unit Context

- Functional goal: load/validate the exFAT upcase table and provide stable name-folding and hash services for later lookup and namespace operations.
- Final architectural owner: `ExfatFs`.
- Expected landing form: owner-internal state (`UpcaseTable`) plus owner methods.
- Interfaces served: future `EXR-FS-OPEN-22`, `EXR-DIR-OPS-23`, and `EXR-NAMESPACE-29`.

## Required Resolution Questions

- Point to the accepted architect artifact and preserve the owner-first boundary.
- Specify the validated `UpcaseTable` state, table validation inputs, and name-folding/hash operations.
- Specify how this unit consumes raw `Upcase` candidates from `DirectoryEngine` without becoming directory scanning.
- Decide whether `02_designer_async.md` is needed; if omitted, record why in `01_designer_core.md`.
- Define checker-owned ktest obligations for validation, folding, hash stability, and boundary non-widening.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/upcase_table.rs`
- Read-only inspection commands inside `/home/halifuda/asterinas` are allowed.

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/03_designer_ktest.md`

## Forbidden Files

- Production code, `COMPONENT_INDEX.md`, main-agent handoffs, and other components' artifacts.

## Required Inputs

- Role-scoped protocol files: `COMMON_SUBAGENT.md`, `DESIGNER.md`.
- Accepted architect artifact: `EXR-UPCASE-20/00_architect.md`.

## Semantic Prior Inputs

- Use accepted architect artifact and accepted dentry/name boundaries only. Do not read Linux source.

## Integration Prior Inputs

- Use `ASTERINAS_ARCHITECT_PRIORS.md` profile `I-DESIGN`.
- Treat legacy `exfat/upcase_table.rs` as integration context only, not as a boundary template if it conflicts with owner-first constraints.

## Workflow Prior Inputs

- Command-free designer lane. May overlap with `EXR-BITMAP-21` designer and active reviewer/checker lanes because write sets are disjoint.

## Quality Prior Inputs

- Use `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-DESIGN`.

## Temporary Interfaces And Exit Plan

- No temporary production surface is expected unless explicitly justified with an exit plan.
- Mount-time installation belongs to `EXR-FS-OPEN-22`.

## Helper Justification

- Upcase helpers must be justified by `ExfatFs` upcase-table ownership and named later consumers.

## Allowed Commands

- Read-only shell commands only. No build/test/QEMU/format commands.

## Parallelism Classification

- Lane class: command-free.
- May overlap with: `EXR-BITMAP-21` designer and active reviewer/checker lanes.

## Execution Environment

- Host read-only inspection from `/home/halifuda/asterinas`.

## Execution Lock

- No execution lock is needed.

## Stop Condition

- Stop after writing the assigned designer artifact set. If `02_designer_async.md` is omitted, explicitly record why in `01_designer_core.md`.

## Escalation Rule

- If the upcase boundary cannot be specified without directory scanning, namespace mutation, or mount/open sequencing, report the gap instead of widening.
