<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-20-20260407-1110-ARCH`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-20/20260407-1110-architect-packet.md`
- Supersedes: None
- Role: `architect`
- Component: `EXR-UPCASE-20`
- Phase: `architect`
- Authorizing main agent: `main-agent`
- Date: `2026-04-07 11:10 CST`

## Goal

- Produce the architect artifact for `EXR-UPCASE-20`: the owner-first boundary for `ExfatFs`-owned upcase-table runtime state and name-folding/hash services, without absorbing directory streaming, VFS directory operations, namespace mutation, or mount/open sequencing.

## Architectural Unit Context

- Functional goal: load/validate the exFAT upcase table and provide stable name-folding and hash services for later lookup and namespace operations.
- Final architectural owner: `ExfatFs`.
- Expected landing form: owner-internal state (`UpcaseTable`) plus owner methods.
- Parent unit: none. This is a tracked functional unit in the live board.
- Interfaces served: future `EXR-FS-OPEN-22`, `EXR-DIR-OPS-23`, and `EXR-NAMESPACE-29`.

## Required Resolution Questions

- What is the upcase-table owner boundary, and what stays in `DirectoryEngine` or later name/namespace owners?
- How should the unit consume `DirectoryEngine` singleton upcase candidates without becoming directory scanning itself?
- Which services are stable now: table loading/validation, UTF-16 case folding, and exFAT name hash support?
- What work slices are safe before `EXR-FS-OPEN-22` wires mount sequencing?
- What file landing zones and future collision risks should the designer know?

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-RESET/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/upcase_table.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/dentry.rs`
- Read-only inspection commands inside `/home/halifuda/asterinas` are allowed.

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/00_architect.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Production code under `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/`
- Other role artifacts outside the write set.

## Required Inputs

- Role-scoped protocol files: `COMMON_SUBAGENT.md`, `ARCHITECT.md`.
- Board reset artifact: `WORKSPACE-ARCH-RESET/00_architect.md`.
- Accepted `EXR-DIR-ENGINE-19` architect/designer artifacts.

## Semantic Prior Inputs

- Use `linux-exFAT-implementation-summary.md` topic “Upcase table and charset behavior” for orientation only.
- Legacy Asterinas `exfat/upcase_table.rs` and name-handling references are integration context, not a license to widen into VFS lookup.
- No direct Linux source reads are authorized. If exact Linux NLS behavior is needed, stop and report.

## Integration Prior Inputs

- Use `ASTERINAS_ARCHITECT_PRIORS.md` profile `I-ARCH`, especially mount/open sequence and directory/runtime owner context.
- Treat `DirectoryEngine` as the source of raw upcase singleton candidates, not as part of this component.

## Workflow Prior Inputs

- Command-free architect lane.
- May overlap with `EXR-BITMAP-21` architect and runtime checker lanes because write sets are disjoint.
- Workflow priors may shape work slices only after owner and upcase-service boundary are resolved.

## Quality Prior Inputs

- Use `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-ARCH`.
- Keep the service under `ExfatFs`; do not create an ownerless name-policy helper.

## Temporary Interfaces And Exit Plan

- No production temporary interface is authorized in this architect pass unless the artifact names the future owner/removal condition.

## Helper Justification

- Any helper-like surface must be justified by `UpcaseTable` ownership and future consumers, not by packet convenience.

## Allowed Commands

- Read-only shell commands only.
- No build/test/QEMU/format commands.

## Parallelism Classification

- Lane class: command-free.
- May overlap with: `EXR-BITMAP-21` architect, pending reviewer packets, and checker lanes with disjoint write sets.
- Known conflicts: none beyond forbidden write sets.

## Execution Environment

- Host read-only inspection from `/home/halifuda/asterinas`.

## Execution Lock

- No execution lock is needed.

## Stop Condition

- Stop after writing `components/EXR-UPCASE-20/00_architect.md`.

## Escalation Rule

- If the upcase-table boundary cannot be defined without directory operations, namespace mutation, or mount/open sequencing, stop and report the missing dependency instead of widening the component.
