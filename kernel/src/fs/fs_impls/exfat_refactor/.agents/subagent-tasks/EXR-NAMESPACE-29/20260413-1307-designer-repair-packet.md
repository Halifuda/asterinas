<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-NAMESPACE-29-20260413-1307-DESIGN-REPAIR`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-NAMESPACE-29/20260413-1307-designer-repair-packet.md`
- Supersedes: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-NAMESPACE-29/20260412-2134-designer-packet.md`
- Role: `designer`
- Component: `EXR-NAMESPACE-29`
- Phase: `designer repair`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 13:07 CST`

## Goal

- Repair the current `EXR-NAMESPACE-29` designer set so later creator work consumes the new `EXR-CHARSET-32` validated converted-name boundary instead of parsing raw `&str` names locally. The revised spec must explicitly pin the handoff from `&str` -> converted UTF-16 value -> `EXR-UPCASE-20` fold/hash -> namespace mutation.

## Architectural Unit Context

- Functional goal: `ExfatInode` namespace mutation
- Final architectural owner: `ExfatInode`
- Expected landing form: owner methods plus owner-private helpers in `inode.rs`
- Architect artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/00_architect.md`

## Required Resolution Questions

- Replace the current "canonicalize the name through `ExfatFs`" wording with a creator-ready handoff that consumes the validated converted-name value from `EXR-CHARSET-32` first and only then calls `EXR-UPCASE-20` fold/hash services on its UTF-16 units.
- State exactly what `EXR-NAMESPACE-29` consumes from `EXR-CHARSET-32`, what remains in `EXR-UPCASE-20`, and what still belongs to `DirectoryEngine`, `Allocator`, and opened-inode publication.
- Keep `EXR-NAMESPACE-29` inode-owned and do not reopen architect ownership.
- Keep volume-label control out of this row and keep sync ordering in `EXR-SYNC-31`.
- Repair the checker obligations so later tests prove the charset boundary is consumed rather than bypassed.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-POST28/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs/vfs/fs_apis/inode.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/03_designer_ktest.md`

## Forbidden Files

- production code
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- creator, checker, advisor, and reviewer artifacts for any component

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/designer.md`

## Semantic Prior Inputs

- `ExfatInode` remains the namespace-mutation owner.
- `EXR-CHARSET-32` now owns external `&str` to validated UTF-16 conversion.
- `EXR-UPCASE-20` remains the only owner of fold/hash over converted UTF-16 units.
- `DirectoryEngine`, `Allocator`, and opened-inode publication remain consumed owners exactly as before.
- `EXR-SYNC-31` remains the downstream owner of persistence ordering.

## Integration Prior Inputs

- This is a repair, not a redesign of the row's final owner.
- The current designer set still implies direct canonicalization from raw `&str` through `ExfatFs`; repair that so creator work does not bypass the new converted-name boundary.
- The repaired spec should allow `EXR-VOLLABEL-35` to reuse `EXR-CHARSET-32` without pulling label control into namespace mutation.

## Workflow Prior Inputs

- Command-free designer repair lane.
- Stay designer-only; do not implement code or schedule follow-up work.
- It is acceptable to leave the current architect artifact untouched if the repair can stay fully within the existing owner boundary.

## Quality Prior Inputs

- Use the designer-role quality slice from `$exfat-subagent-workflow`.
- Replace ambiguous or outdated name-preflight wording with one concrete owner-first handoff.
- Keep helper and temporary-seam surfaces explicitly subordinate to `ExfatInode`.

## Temporary Interfaces And Exit Plan

- Do not authorize a namespace manager, label-control path, or generic text helper in this repair pass.
- If a temporary seam remains necessary, name its future owner and removal condition explicitly.

## Helper Justification

- Allowed helper surfaces are owner-private namespace preflight helpers that consume:
  - a validated converted-name value from `EXR-CHARSET-32`,
  - fold/hash behavior from `EXR-UPCASE-20`,
  - and the existing mutation/publication owners.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with:
  - the active `EXR-SYNC-31` designer lane

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after rewriting:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/01_designer_core.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/02_designer_async.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/03_designer_ktest.md`

## Escalation Rule

- If the current architect boundary plus `EXR-CHARSET-32` still do not support a clean namespace preflight handoff without reopening architect ownership, report the exact missing handshake and stop instead of guessing.
