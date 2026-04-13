<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-CHARSET-32-20260413-1301-ARCHITECT`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-CHARSET-32/20260413-1301-architect-packet.md`
- Supersedes: None
- Role: `architect`
- Component: `EXR-CHARSET-32`
- Phase: `architect`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 13:01 CST`

## Goal

- Define the smallest owner-first `EXR-CHARSET-32` unit that gives `ExfatFs` explicit ownership of VFS-visible exFAT name and label conversion policy without reopening `EXR-UPCASE-20`, `EXR-NAMESPACE-29`, or `EXR-VOLLABEL-35`.

## Architectural Unit Context

- Functional goal: filesystem-owned exFAT name and label conversion service
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-internal service plus possibly a validated converted-name type
- Board authority:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-POST28/00_architect.md`

## Required Resolution Questions

- What is the smallest architecturally real unit that covers external name/label conversion for exFAT under Asterinas VFS?
- How should `EXR-CHARSET-32` consume, but not reopen, the accepted `EXR-UPCASE-20` fold/hash boundary?
- What should later rows receive from this component:
  - UTF-16 units only,
  - a validated converted-name type,
  - or a small owner-private conversion context?
- Should this row explicitly support UTF-8-only external names for now, or architect room for Linux-style optional NLS/UTF-8 policy while still allowing the designer to close Linux NLS parity as a non-goal?
- Which later rows should consume this boundary directly, especially `EXR-NAMESPACE-29` and `EXR-VOLLABEL-35`?
- Which concerns must stay out:
  - upcase-table installation
  - namespace mutation
  - volume-label mutation
  - directory scanning
  - sync ordering

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/topaz-bridge-20260413-1256-tail-reshape.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-POST28/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/linux/fs/exfat/exfat_fs.h`
- `/home/halifuda/linux/fs/exfat/nls.c`
- `/home/halifuda/linux/fs/exfat/namei.c`
- `/home/halifuda/linux/fs/exfat/file.c`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/00_architect.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- all production code
- all non-architect artifacts for any component

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/architect.md`

## Semantic Prior Inputs

- `Microsoft-exFAT-spec.md` remains the on-disk authority.
- `linux-exFAT-implementation-summary.md` plus the authorized Linux files may be used where external name conversion, UTF-8/NLS policy, or label-string handling need implementation-shaped guidance.
- `EXR-UPCASE-20` already owns the validated volume upcase table and folded UTF-16 hashing boundary. This row must consume that service, not reopen it.

## Integration Prior Inputs

- The current refactor uses raw `name.encode_utf16()` at lookup sites and `String::from_utf16()` for visible names in `inode.rs`.
- The board now blocks `EXR-NAMESPACE-29` on an explicit charset row.
- `EXR-VOLLABEL-35` is planned and should likely consume the same conversion boundary.
- Asterinas VFS path/inode methods expose Rust `&str` names, so the external boundary is not Linux's byte-string API, but Linux code remains relevant for policy and edge cases.

## Workflow Prior Inputs

- Command-free architect lane.
- This packet is for one architect artifact only.
- Recommend stable tracked-unit ownership, not creator slicing or packet convenience cuts.

## Quality Prior Inputs

- Use the architect-role quality slice from `$exfat-subagent-workflow`.
- Reject generic helper-module drift.
- Call out if `fs.rs` is the likely shared landing zone with later `EXR-VOLLABEL-35` or `EXR-SYNC-31`.

## Temporary Interfaces And Exit Plan

- Do not edit `COMPONENT_INDEX.md`.
- Do not repair `EXR-NAMESPACE-29` yet.
- Stop after producing the architect artifact for `EXR-CHARSET-32`.

## Helper Justification

- This row may define a validated converted-name type if doing so keeps name conversion from leaking into every later row, but only if that type is justified as a stable filesystem-visible or owner-visible boundary rather than a packet convenience helper.

## Allowed Commands

- Read-only shell inspection commands under:
  - `/home/halifuda/asterinas`
  - `/home/halifuda/linux/fs/exfat`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with:
  - `EXR-SYNC-31` architect/design planning

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/00_architect.md`

## Escalation Rule

- If the current priors plus the authorized Linux files are still insufficient to justify a stable charset/name-conversion owner without widening into namespace or label mutation, report the exact missing boundary and stop instead of guessing.
