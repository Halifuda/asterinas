<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-BOOT-34-20260413-1421-DESIGN`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BOOT-34/20260413-1421-designer-packet.md`
- Supersedes: None
- Role: `designer`
- Component: `EXR-BOOT-34`
- Phase: `designer`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 14:21 CST`

## Goal

- Produce the split designer artifact set for `EXR-BOOT-34` so later creator work can implement `ExfatFs` boot-region fallback and persistent boot-flag policy without guessing about mount/open ownership, sync handoff, or checker obligations.

## Architectural Unit Context

- Functional goal: `ExfatFs` backup-boot fallback and persistent boot-region policy
- Final architectural owner: `ExfatFs`
- Expected landing form: owner methods plus mount/open policy helpers
- Architect artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/00_architect.md`

## Required Resolution Questions

- Specify the smallest `ExfatFs`-owned policy surface that decides:
  - whether mount/open trusts the validated primary boot facts or a backup-boot fallback path,
  - when `VolumeDirty` must be published as persistent boot-region intent,
  - when `ClearToZero` must be cleared before later volume mutation,
  - and how `PercentInUse` is treated as a bounded policy input or a non-owning observation.
- State exactly how `EXR-FS-OPEN-22` consumes this policy result without reabsorbing boot-region ownership, and how `EXR-SYNC-31` later consumes only the resulting dirty boot intent rather than the decision logic itself.
- Keep boot parsing and checksum validation out of this row: `EXR-BOOT-01` remains the only validated primary-boot fact owner.
- Keep the row out of admin/control drift: no volume-label control, no direct I/O, no FAT-attribute ioctls, no trim/discard, and no forced shutdown.
- Define narrow creator and checker obligations so later work does not guess where boot fallback ends and where mount/open orchestration or sync ordering begins.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-01/01_designer_spec.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYNC-31/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYNC-31/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/file_system.rs`
- `/home/halifuda/linux/fs/exfat/exfat_fs.h`
- `/home/halifuda/linux/fs/exfat/super.c`
- `/home/halifuda/linux/fs/exfat/file.c`
- `/home/halifuda/linux/fs/exfat/inode.c`
- `/home/halifuda/linux/fs/exfat/namei.c`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/03_designer_ktest.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- production code
- creator, checker, advisor, and reviewer artifacts for any component

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/designer.md`

## Semantic Prior Inputs

- `EXR-BOOT-34` is the smallest real `ExfatFs` policy unit above validated boot facts.
- `EXR-BOOT-01` remains the only owner of validated primary boot parsing and checksum verification.
- `EXR-FS-OPEN-22` remains the only owner of mount/open sequencing and root publication; it must consume this row's decision rather than absorb it.
- `EXR-SYNC-31` remains the only filesystem-wide flush-ordering owner; it may later consume dirty boot intent from this row but must not own fallback or flag-decision policy.
- `VolumeDirty` and `ClearToZero` are persistent boot-region outputs, not generic inode metadata.
- `PercentInUse` may stay observational if no real policy use survives the design pass.

## Integration Prior Inputs

- The current refactor code validates and normalizes only the primary boot region through `read_primary_super_block(...)`.
- `fs.rs` currently owns mount/open state and the placeholder `sync()` seam, but has no backup-boot compare/fallback or persistent boot-flag policy yet.
- Linux keeps boot-buffer state on the filesystem owner and updates persistent volume flags through dedicated helpers in `super.c`; this row should reuse only the architectural lesson, not clone Linux's exact surface.
- The board intentionally tracks backup-boot fallback and boot-flag policy as a separate row so they are not silently smuggled into `EXR-FS-OPEN-22` or `EXR-SYNC-31`.

## Workflow Prior Inputs

- Command-free designer lane.
- Artifact-only planning; may overlap with main-agent bookkeeping because the write sets are disjoint.
- Produce the standard split artifact set: core, async, and ktest.
- Stay designer-only; do not drift into creator implementation details beyond creator-ready obligations.

## Quality Prior Inputs

- Use the designer-role quality slice from `$exfat-subagent-workflow`.
- Keep helper shape explicitly owner-private to `ExfatFs`.
- Reject drift into a generic recovery shell, sync bucket, or mount/open clone.
- Make the boot-policy output shape explicit enough that later creator work can land in `fs.rs` without reopening ownership.

## Temporary Interfaces And Exit Plan

- Do not authorize a second boot parser, a background recovery worker, or any new public boot-policy API.
- If the only clean design requires a new validated value type or a reopened board split, stop and report the exact missing boundary instead of guessing.

## Helper Justification

- Allowed helper surfaces are owner-private helpers that:
  - compare or select mount-time boot-region facts,
  - expose persistent boot-flag intent to the future sync owner,
  - and let mount/open consume a stable policy result without becoming the policy owner.
- They must remain subordinate to `ExfatFs`.

## Allowed Commands

- Read-only shell inspection commands under:
  - `/home/halifuda/asterinas`
  - `/home/halifuda/linux/fs/exfat`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with:
  - main-agent artifact bookkeeping only

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/01_designer_core.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/02_designer_async.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/03_designer_ktest.md`

## Escalation Rule

- If the architect artifact plus current priors are still insufficient to specify boot fallback and persistent boot-region policy cleanly without reopening `EXR-BOOT-01`, `EXR-FS-OPEN-22`, or `EXR-SYNC-31`, report the exact missing handshake and stop instead of guessing.
