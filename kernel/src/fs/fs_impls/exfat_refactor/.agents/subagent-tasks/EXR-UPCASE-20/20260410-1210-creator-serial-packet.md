<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-20-20260410-1210-CREATE-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-20/20260410-1210-creator-serial-packet.md`
- Supersedes: None
- Role: `creator`
- Component: `EXR-UPCASE-20`
- Phase: `serial creator`
- Authorizing main agent: `main-agent`
- Date: `2026-04-10 12:10 CST`

## Goal

- Implement `ExfatFs`-owned validated upcase-table state, owner-local folding, and exFAT name-hash services in `fs.rs` without widening into directory traversal, mount sequencing, or a generic text helper module.

## Architectural Unit Context

- Functional goal: `ExfatFs`-owned upcase table state plus canonicalization services
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-private `UpcaseTable` state and owner methods in `fs.rs`
- Parent unit: `EXR-DIR-ENGINE-19`
- Interfaces served: later lookup, namespace, and mount/open work that needs canonicalized UTF-16 names

## Required Resolution Questions

- Add owner-private validated upcase-table state under `ExfatFs`.
- Implement validated publication of a raw `Upcase` candidate before folding or hashing can use it.
- Implement UTF-16 folding through the installed table.
- Implement exFAT name hashing from folded UTF-16 bytes.
- Keep all table logic owner-local in `fs.rs` and avoid a new generic text helper namespace.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/10_creator_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Sibling component artifacts.

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/creator.md`

## Semantic Prior Inputs

- Use the accepted designer constraints only. Do not reopen Linux or Microsoft exFAT behavior beyond the designer-approved upcase-table and name-hash semantics.

## Integration Prior Inputs

- `EXR-DIR-ENGINE-19` is accepted and now provides the raw `Upcase` singleton boundary; consume that boundary without reintroducing directory scanning here.
- `EXR-BITMAP-21` remains out of scope even though it is another `ExfatFs` owner-state row.

## Workflow Prior Inputs

- Command-free creator lane.
- This is the next loop's only creator round.
- Do not run compile or test commands; checker will own executable verification and the required local ktests.

## Quality Prior Inputs

- Use the creator-role quality slice from `$exfat-subagent-workflow`.
- Keep publication atomic and the installed table immutable after validation.

## Temporary Interfaces And Exit Plan

- Do not widen this pass into mount/open sequencing.
- Do not create a fallback locale table, generic Unicode helper module, or public cache surface for raw table fields.

## Helper Justification

- Small owner-private helpers inside `fs.rs` are allowed when they keep validation, folding, and hashing readable.
- Reject helpers whose main effect is to invent a reusable generic text subsystem.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free production edit`
- May overlap with command-free lanes only
- Known conflicts:
  - `fs.rs`

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/10_creator_serial.md`

## Escalation Rule

- If the implementation requires edits outside `fs.rs` or suggests that the component boundary itself is underspecified, report that and stop.
