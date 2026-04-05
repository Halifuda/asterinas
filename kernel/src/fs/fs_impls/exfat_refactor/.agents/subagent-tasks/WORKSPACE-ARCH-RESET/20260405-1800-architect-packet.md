<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `WORKSPACE-ARCH-RESET-20260405-1800`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/WORKSPACE-ARCH-RESET/20260405-1800-architect-packet.md`
- Supersedes: None
- Role: `architect`
- Component: `WORKSPACE-ARCH-RESET`
- Phase: `architect`
- Authorizing main agent: `main-agent`
- Date: `2026-04-05 18:00 CST`

## Goal

- Produce a clean-slate architect proposal that fully rethinks the current exFAT task board under the new owner-first protocol. You may merge, split, rename, or remove existing planned units, and you may challenge even the current accepted unit boundaries if they do not look architecturally real under the new rules. Do not edit the official task board yourself; instead write a replacement-ready architect proposal that the main agent can use to rebuild [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md).

## Architectural Unit Context

- Functional goal: redesign the tracked-unit decomposition for `exfat_refactor` so the board can become the canonical working record for future implementation under the new owner-first architect rules.
- Final architectural owner when known: not a production-code owner; this is a scheduler-owned planning task. Your job is to determine what the production architectural owners and tracked functional units should be.
- Expected landing form: one architect proposal artifact that contains:
  - a proposed replacement unit graph,
  - proposed unit owners and boundary kinds,
  - a mapping from current ids to proposed ids or retirement decisions,
  - and a migration recommendation for rebuilding `COMPONENT_INDEX.md`.
- Parent unit: none. This is a workspace-wide re-architecture pass.
- Interfaces or higher-level functions ultimately served:
  - future VFS-facing `ExfatFs` / `ExfatInode`-style integration,
  - any justified internal service/process boundaries,
  - and a task board that is usable for future creator/designer/checker work.

## Required Resolution Questions

- Unit-definition questions:
  - What should the tracked functional units be for the refactor going forward?
  - Which currently tracked units are architecturally real, and which are packet-convenience cuts?
  - Which accepted low-level units should remain intact, and which should be merged into larger owners or otherwise reclassified for future planning?
- Owner-definition questions:
  - What are the stable final owners that should organize the future refactor?
  - Which planned behaviors belong to trait carriers, internal runtime owners, validated value types, or other stable service/process boundaries?
- Work-slice and parallel-wave questions:
  - Given the real units above, how should the future board still preserve creator parallelism and disjoint write-set opportunities?
  - Which units should be tracked separately, and which should instead become work slices inside a larger unit?

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/PROTOCOL.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/stone-lantern-20260405-1721-rollback-baseline.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/TASK_PACKET_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/file_system.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode_ext.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/`
- Read-only inspection commands inside `/home/halifuda/asterinas` are allowed when needed to understand current boundaries.

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-RESET/00_architect.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Production code under `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/`
- Any designer, creator, checker, advisor, or reviewer artifact outside the assigned output file

## Required Inputs

- The role-scoped protocol files that accompany this packet:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- Scheduler protocol is also intentionally included because this is a scheduler-owned task-board redesign task:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/PROTOCOL.md`
- Required prior records:
  - current rollback baseline handoff
  - current `COMPONENT_INDEX.md`
  - current protocol/templates already modified to the new owner-first semantics

## Semantic Prior Inputs

- Full semantic prior set is authorized:
  - `Microsoft-exFAT-spec.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
- Intended precedence:
  - `Microsoft-exFAT-spec.md` for normative exFAT semantics
  - `linux-exFAT-implementation-summary.md` for preferred implementation guidance when the spec leaves room
- Use these to judge what real filesystem functions exist and which boundaries are semantically coherent.

## Integration Prior Inputs

- Authorized integration priors:
  - full `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`
  - `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/file_system.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode_ext.rs`
  - current `exfat_refactor` code layout
  - legacy `kernel/src/fs/fs_impls/exfat/` only as integration context, not as semantic authority
- Integration facts that matter:
  - trait carriers and runtime owners must eventually be concrete and stable,
  - not every real unit is a VFS trait method, but every real unit must still serve a stable function and stable owner,
  - creator parallelism matters, but file layout and work slicing must follow real boundaries rather than invent them.

## Workflow Prior Inputs

- Workflow constraints that matter:
  - future creator parallelism is important and should be preserved deliberately,
  - we do not want most future work trapped in one or two giant files,
  - packet-sized work slices and disjoint write sets remain important,
  - the current board is a rollback baseline and should not be treated as canon,
  - you must not directly edit `COMPONENT_INDEX.md`; you are producing a replacement-ready architect proposal only.
- Workflow priors may shape work slices and future file/layout recommendations only after semantic and integration questions are resolved.

## Quality Prior Inputs

- Use:
  - boundary-level quality guidance from `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - owner clarity,
  - boundary clarity,
  - avoiding packet-convenience abstractions,
  - preserving room for clean future file/module organization.
- Out of scope:
  - creator-local naming or formatting micro-decisions.

## Prior Delivery Notes

- This packet is intentionally broad semantically but narrow operationally: you are redesigning the task board architecture, not implementing code or editing scheduler-owned files.
- Open questions are intentionally split as:
  - semantic: what real filesystem functions exist and belong together,
  - integration: who should own those functions in the finished system,
  - workflow: how to preserve creator parallelism without inventing fake architectural boundaries.

## Temporary Interfaces And Exit Plan

- No production temporary interface is authorized because this is a planning-only task.
- Your proposal may recommend temporary construction seams for future work, but if so you must name their future owner or removal condition explicitly.

## Helper Justification

- No new production helpers are authorized.
- In the proposal, call out any currently implied helper-only boundaries that should instead remain owner-internal.

## Allowed Commands

- Read-only shell commands for inspection inside `/home/halifuda/asterinas`
- No build, test, or QEMU commands

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - other command-free lanes with disjoint write sets
- Known conflicts:
  - none beyond the assigned write set

## Execution Environment

- Host or Docker: host-side read-only inspection is sufficient
- Required command prefix: none
- Required working directory: `/home/halifuda/asterinas`
- Isolation notes: command-free only
- This task must not run compile or runtime commands.

## Execution Lock

- No execution lock is needed because this task is command-free.

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-RESET/00_architect.md`.
- Do not edit `COMPONENT_INDEX.md`.
- Do not write designer artifacts, packets, or main-agent handoff files.

## Escalation Rule

- If the task still appears too broad for one architect pass, do not try to solve it by shrinking scope silently. Instead, write the best full-board proposal you can, name the remaining irreducible uncertainties explicitly, and stop.
