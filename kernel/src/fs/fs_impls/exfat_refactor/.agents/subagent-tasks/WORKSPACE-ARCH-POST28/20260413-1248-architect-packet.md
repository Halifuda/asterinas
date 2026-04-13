<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `WORKSPACE-ARCH-POST28-20260413-1248-ARCHITECT`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/WORKSPACE-ARCH-POST28/20260413-1248-architect-packet.md`
- Supersedes: None
- Role: `architect`
- Component: `WORKSPACE-ARCH-POST28`
- Phase: `architect`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 12:48 CST`

## Goal

- Re-audit the post-`EXR-DENTRY-WRITE-28` exFAT board against the three repository-local priors and the in-tree Linux exFAT implementation, then produce an owner-first architect proposal that:
  - identifies any remaining exFAT functional units that still need explicit tracked modules or explicit non-goal closure,
  - states whether `O_DIRECT` belongs inside a redesigned `EXR-WRITE-30` or in a new downstream module,
  - and recommends a revised post-28 component graph and sequencing, including any redesign of `EXR-NAMESPACE-29`, `EXR-WRITE-30`, and the tail of the board.

## Architectural Unit Context

- Functional goal: scheduler continuity and board reshaping after the acceptance of `EXR-DENTRY-WRITE-28`
- Final architectural owner:
  - proposal-only workspace architect artifact for the main-agent
- Expected landing form:
  - architect proposal artifact only
- Existing reset artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-RESET/00_architect.md`

## Required Resolution Questions

- Starting from repository-local priors first, which exFAT concerns remain outside the current owner-first board even if `EXR-NAMESPACE-29`, `EXR-WRITE-30`, and `EXR-SYNC-31` are all completed?
- Which of those concerns are architecturally real enough to require new tracked modules, rather than being silently folded into current rows?
- Should `O_DIRECT` support be:
  - included inside a redesigned `EXR-WRITE-30`,
  - deferred explicitly to a new downstream module,
  - or closed as a deliberate non-goal?
- Do `EXR-NAMESPACE-29` and `EXR-WRITE-30` need to be re-cut so they stop short of functionality that belongs in later modules?
- What should the revised tail of the board be after `EXR-DENTRY-WRITE-28`, including new modules for any omitted but architecturally real exFAT or Asterinas/VFS integration surfaces?
- Which omitted items may reasonably be tracked as explicit non-goals instead of new modules?

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/amber-delta-20260413-0725-dentry-write-closure.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-RESET/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYNC-31/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/linux/fs/exfat/exfat_fs.h`
- `/home/halifuda/linux/fs/exfat/super.c`
- `/home/halifuda/linux/fs/exfat/file.c`
- `/home/halifuda/linux/fs/exfat/inode.c`
- `/home/halifuda/linux/fs/exfat/namei.c`
- `/home/halifuda/linux/fs/exfat/dir.c`
- `/home/halifuda/linux/fs/exfat/balloc.c`
- `/home/halifuda/linux/fs/exfat/nls.c`
- `/home/halifuda/linux/fs/exfat/misc.c`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-POST28/00_architect.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- all production code
- reviewer, creator, checker, and designer artifacts for existing components

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/architect.md`

## Semantic Prior Inputs

- Treat `Microsoft-exFAT-spec.md` as the normative source for exFAT on-disk semantics.
- Treat `linux-exFAT-implementation-summary.md` plus the authorized Linux source files as the implementation reference where the spec leaves design room or where current-board closure may be incomplete.
- Treat `ASTERINAS_ARCHITECT_PRIORS.md` as binding on local integration owners and VFS/page-cache realities.
- The current board may be incomplete. If a concern is architecturally real and still uncovered, recommend a new module even if it implies a prior omission.

## Integration Prior Inputs

- The current board already accepts through `EXR-DENTRY-WRITE-28`.
- `EXR-NAMESPACE-29` and `EXR-WRITE-30` are still only specified.
- `EXR-SYNC-31` is still planned.
- The current `write_at` and `resize` paths remain unimplemented in production `inode.rs`.
- The user explicitly called out `O_DIRECT` as an omitted surface that should be considered during this board reshape.

## Workflow Prior Inputs

- Command-free architect lane.
- This is a proposal artifact, not a board edit.
- Recommend stable tracked functional units only; do not optimize for fake parallelism.
- If a concern belongs in an explicit non-goal rather than a new tracked module, say so clearly.

## Quality Prior Inputs

- Use the architect-role quality slice from `$exfat-subagent-workflow`.
- Reject packet-convenience modules that do not have a stable final owner and boundary justification.
- Call out likely file-collision zones where creator waves should remain serialized.

## Temporary Interfaces And Exit Plan

- Do not edit `COMPONENT_INDEX.md` directly.
- Do not rewrite existing `EXR-NAMESPACE-29` or `EXR-WRITE-30` artifacts in place.
- Instead, produce one new architect proposal that the main-agent can use to update the board and issue follow-on packets.

## Helper Justification

- This packet allows a proposal-only architect artifact because the question is cross-component board shape, not one ordinary implementation module.
- Any recommended new modules must still name their stable final owner and landing form.

## Allowed Commands

- Read-only shell inspection commands under:
  - `/home/halifuda/asterinas`
  - `/home/halifuda/linux/fs/exfat`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with:
  - main-agent local Linux/prior inspection and later board editing preparation

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-POST28/00_architect.md`

## Escalation Rule

- If the current priors plus the authorized Linux source files are still insufficient to recommend stable owner-first modules, report the exact missing authority and stop instead of guessing.
