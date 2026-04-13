<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-BOOT-34-20260413-1413-ARCHITECT`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BOOT-34/20260413-1413-architect-packet.md`
- Supersedes: None
- Role: `architect`
- Component: `EXR-BOOT-34`
- Phase: `architect`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 14:13 CST`

## Goal

- Define the smallest owner-first `EXR-BOOT-34` unit that gives `ExfatFs` explicit ownership of backup-boot fallback / compare policy and persistent boot-region flag policy without absorbing filesystem-wide sync ordering, volume-label control, trim/discard, or forced-shutdown administration.

## Architectural Unit Context

- Functional goal: backup-boot fallback and persistent boot-region policy
- Final architectural owner: `ExfatFs`
- Expected landing form: owner methods plus mount/open policy helpers
- Board authority:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-POST28/00_architect.md`

## Required Resolution Questions

- What is the smallest architecturally real `ExfatFs` unit that covers:
  - primary-versus-backup boot-region compare or fallback policy at mount/open time
  - persistent policy for `VolumeDirty`
  - persistent policy for `ClearToZero`
  - and the row's stance on `PercentInUse`
  without turning `EXR-BOOT-34` into a generic recovery or sync bucket?
- Which responsibilities should stay inside this row versus remain in:
  - `EXR-BOOT-01` validated primary boot parsing
  - `EXR-FS-OPEN-22` mount/open sequencing
  - `EXR-SYNC-31` filesystem-wide flush ordering
- Should the row own only mount-time fallback decisions plus policy for boot-region flag publication, or also the concrete helper surface that later dirty producers use to mark boot flags dirty?
- Which pieces are real stable boundary outputs for later rows:
  - a mount-time boot-source decision
  - a persistent-flag policy surface
  - a boot-region dirty output later consumed by sync?
- What must stay out:
  - name conversion
  - volume-label control
  - direct I/O
  - trim/discard
  - forced shutdown
  - FAT-attribute ioctls

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/quartz-cascade-20260413-1314-charset-sync-spec-wave.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-POST28/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-01/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-01/01_designer_spec.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYNC-31/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYNC-31/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/linux/fs/exfat/super.c`
- `/home/halifuda/linux/fs/exfat/exfat_fs.h`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/00_architect.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- all production code
- all non-architect artifacts for any component

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/architect.md`

## Semantic Prior Inputs

- `EXR-BOOT-01` already owns validated primary boot parsing; do not reopen that row into fallback policy.
- `EXR-FS-OPEN-22` already owns mount/open sequencing; this row should consume that owner and name the additional policy boundary it still lacks.
- `EXR-SYNC-31` is explicitly flush-ordering only; boot fallback decisions and boot-region control policy must stay outside it.
- Use the Microsoft spec for exact flag semantics and Linux `super.c` for orientation on boot-region and persistent-flag handling, but do not widen this row into Linux-only admin compatibility.

## Integration Prior Inputs

- Current Asterinas code only reads the primary boot region and currently keeps persistent-volume-flag handling narrow in `boot_sector.rs`; architect this row as the missing policy layer above validated boot facts, not as a parser redo.
- Linux currently keeps boot-sector read/verify and persistent `vol_flags` policy close to `super.c`; use that as an owner-shape clue, not as a mandate to copy Linux structure layout.
- Later `EXR-SYNC-31` may consume a boot-region dirty output from this row, but this row should still own the decision and policy boundary for when such output exists.

## Workflow Prior Inputs

- Command-free architect lane.
- This packet is for one architect artifact only.
- Recommend the stable unit boundary and likely creator-slice guidance, but do not schedule or implement.

## Quality Prior Inputs

- Use the architect-role quality slice from `$exfat-subagent-workflow`.
- Reject any split that turns boot fallback into a sync bucket, generic recovery shell, or mount-open clone.
- Call out likely `fs.rs` collision zones for later creator waves.

## Temporary Interfaces And Exit Plan

- Do not edit `COMPONENT_INDEX.md`.
- Do not define designer-level test coverage or lock ordering yet.
- Stop after producing the architect artifact for `EXR-BOOT-34`.

## Helper Justification

- This row may recommend owner-private policy helpers under `ExfatFs` if they are necessary to express boot-source decision or boot-flag policy, but it must not invent a second mount object, recovery manager, or public boot-admin service.

## Allowed Commands

- Read-only shell inspection commands under:
  - `/home/halifuda/asterinas`
  - `/home/halifuda/linux/fs/exfat`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with:
  - `EXR-CHARSET-32` checker

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/00_architect.md`

## Escalation Rule

- If the current priors still do not support a stable split between boot fallback / boot-flag policy and mount-open or sync ownership, report the exact unresolved boundary and stop instead of guessing.
