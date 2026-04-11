<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-INODE-CACHE-18-20260410-1130-CREATE-REPAIR`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CACHE-18/20260410-1130-creator-repair-packet.md`
- Supersedes: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CACHE-18/20260410-1058-creator-serial-packet.md`
- Role: `creator`
- Component: `EXR-INODE-CACHE-18`
- Phase: `serial creator repair`
- Authorizing main agent: `main-agent`
- Date: `2026-04-10 11:30 CST`

## Goal

- Repair the local `fs.rs` build failure surfaced by the serial checker so the opened-inode cache ktests can compile and the next checker rerun can reach executable evidence.

## Architectural Unit Context

- Functional goal: `ExfatFs`-owned opened-inode table keyed by the owner-private `InodeKey`, with the root special case outside the ordinary keyspace.
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-internal state plus validated `InodeKey` in `fs.rs`
- Parent units: `EXR-FS-CORE-16`, `EXR-INODE-CORE-17`
- Interfaces served: later `EXR-FS-OPEN-22` and lookup/open reuse paths

## Required Resolution Questions

- Fix the checker-blocking borrow-of-moved-value error in the `fs.rs` test helper without widening scope outside `EXR-INODE-CACHE-18`.
- Preserve the three checker-owned local ktests and the opened-inode cache behavior they exercise.
- Keep the root special case as a separate owner-private slot rather than synthesizing an ordinary key.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/12_creator_serial_repair.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Sibling component artifacts.

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/creator.md`
- Creator and checker artifacts listed above

## Semantic Prior Inputs

- Use the accepted designer constraints only. Do not reopen Linux or Microsoft exFAT behavior.

## Integration Prior Inputs

- Treat the current `directory.rs` checker-owned fixes as out of scope; this repair is only for the remaining `fs.rs` blocker.
- Preserve the checker-added ktests in `fs.rs` unless a local rename is strictly required to keep them compiling.

## Workflow Prior Inputs

- Command-free creator repair.
- This is the only creator round opened in the current loop.
- Do not run compile or test commands; the follow-on checker pass owns executable verification.

## Quality Prior Inputs

- Use the creator-role quality slice from `$exfat-subagent-workflow`.
- Prefer the smallest local repair that restores buildability for the checker rerun.

## Temporary Interfaces And Exit Plan

- Preserve the temporary `root_inode()` seam for `EXR-FS-OPEN-22`.
- Do not add public cache helpers or any synthetic root key.

## Helper Justification

- Local helper reshaping is allowed only if needed to eliminate the checker-blocking move/borrow issue while preserving the owner-private cache boundary.

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

- None; command-producing work is not authorized in this packet

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/12_creator_serial_repair.md`

## Escalation Rule

- If the repair requires edits outside `fs.rs` or exposes a broader design problem rather than a local compile fix, report that and stop.
