<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-20-20260410-1050-DESIGN`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-20/20260410-1050-designer-packet.md`
- Supersedes: None
- Role: `designer`
- Component: `EXR-UPCASE-20`
- Phase: `design`
- Authorizing main agent: `main-agent`
- Date: `2026-04-10 10:50 CST`

## Goal

- Turn the accepted `EXR-UPCASE-20` architect boundary into a bounded designer artifact set that specifies `ExfatFs`-owned upcase-table state and name-folding/hash services without widening into directory traversal, mount sequencing, or namespace behavior.

## Architectural Unit Context

- Functional goal: load and validate the exFAT upcase table, then provide stable case-folding and exFAT name-hash services for later lookup and namespace work.
- Final architectural owner: `ExfatFs`.
- Expected landing form: owner-internal state (`UpcaseTable`) plus owner methods.
- Parent unit: none.
- Interfaces served: future `EXR-FS-OPEN-22`, `EXR-DIR-OPS-23`, and `EXR-NAMESPACE-29`.

## Required Resolution Questions

- Refine the architected owner boundary into an implementable spec without reopening the owner question.
- State how the validated table is stored under `ExfatFs`.
- State which folding and name-hash services belong in this unit and which nearby logic stays out.
- Decide whether a dedicated async artifact is needed; if not, say why explicitly.
- Define checker-owned test obligations for table validation, folding behavior, and name-hash behavior.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/03_designer_ktest.md`

## Forbidden Files

- Production code.
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Sibling component artifacts.

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/designer.md`
- Designer spec template.
- Based-on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/00_architect.md`

## Semantic Prior Inputs

- Primary semantic priors:
  - selected upcase-table semantics from `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md` topic on upcase-table and charset behavior
- Precedence:
  - Microsoft exFAT semantics first
  - Linux summary second

## Integration Prior Inputs

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md` profile `I-ARCH`
- The accepted architect artifact is the primary owner-boundary source.

## Workflow Prior Inputs

- Command-free designer lane.
- This lane may overlap with:
  - `EXR-BITMAP-21` designer
  - `EXR-FS-CORE-16` reviewer
  - `EXR-INODE-CORE-17` reviewer
- No creator work is authorized in this packet.

## Quality Prior Inputs

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-DESIGN`

## Temporary Interfaces And Exit Plan

- If a temporary staging surface is unavoidable, name the later owner or removal condition explicitly.
- Do not authorize a free helper namespace or mount-sequencing placeholder in this packet.

## Helper Justification

- Do not specify short helper APIs or field accessors unless the spec names the expected caller or boundary that requires them now.

## Allowed Commands

- Read-only shell inspection only.
- No build, test, format, Docker, KVM, or QEMU commands.

## Parallelism Classification

- Lane class: `command-free`
- May overlap with command-free lanes that keep disjoint artifact write sets.
- Known conflicts: none beyond its own artifact files.

## Execution Environment

- Host read-only inspection and artifact edits under `/home/halifuda/asterinas`.

## Execution Lock

- No execution lock is needed.

## Stop Condition

- Stop after writing the required designer artifact set for `EXR-UPCASE-20`.

## Escalation Rule

- If the component is still too coarse for one creator pass or the architect artifact leaves an unresolved owner question, report that and stop.
