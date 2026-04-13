<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Async

## Metadata

- Component ID: `EXR-BOOT-34`
- Title: `ExfatFs` Boot-Policy Publication Serialization
- Status: `Specified`
- Author: designer
- Date: `2026-04-13`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BOOT-34/20260413-1421-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/00_architect.md`

## Scope

- In scope:
  - Define the serialization boundary for publishing the boot-region policy result inside `ExfatFs`.
  - Make the boot-source choice and the persistent boot-intent snapshot linearizable to later mount/open and sync consumers.
  - Keep the policy result owner-private so `EXR-FS-OPEN-22` and `EXR-SYNC-31` can consume it without owning it.
  - Preserve `ClearToZero` as a pre-mutation guard rather than a transient publication artifact.
- Out of scope:
  - Any background recovery worker.
  - Any deferred boot repair queue.
  - A public boot-policy API.
  - A new boot parser or checksum path.
  - Direct I/O, label control, trim/discard, forced shutdown, or FAT-attribute ioctls.

## Serialization Contract

- Shared boundaries involved:
  - The owner-private boot-policy snapshot.
  - The owner-private persistent boot-intent snapshot.
  - The mount/open consumer state in `EXR-FS-OPEN-22`.
  - The later sync consumer state in `EXR-SYNC-31`.
- Linearization rule:
  - Callers must observe either the pre-policy state or the fully published boot-policy state.
  - The trusted boot source and the persistent boot intent must become visible together from the caller's point of view.
- Publication order:
  - First, accept already validated boot facts from the owner boundary.
  - Second, choose the trusted boot source.
  - Third, record `VolumeDirty` and `ClearToZero` intent in owner-private state.
  - Fourth, expose the ready mount/open state to `EXR-FS-OPEN-22`.
  - Fifth, expose only the dirty boot intent to `EXR-SYNC-31`.
- Reuse rule:
  - Once a boot source is selected for a mounted filesystem instance, later mount/open calls use the same published result until remount.
  - The policy result is a snapshot, not a moving target.
- Stability rule:
  - `PercentInUse` remains observational and must not perturb the publication order unless a later owner proves a bounded policy use.

## Lock-Order Expectations

- Owner gate before exposure:
  - Acquire the owner-private serialization boundary before publishing the policy result.
- Decision before publication:
  - Complete the trusted boot-source decision before exposing mount/open readiness.
- Publication before sync:
  - Do not let sync observe a dirty boot intent that was not published as part of the same owner-private state.
- Pre-mutation before later writes:
  - `ClearToZero` must remain present until a later mutating path clears it for the first boot-region writeback that needs that guarantee.

## Forbidden Interleavings

- Do not let mount/open see one boot source while sync drains intent from another.
- Do not let `VolumeDirty` be cleared before the published policy result exists.
- Do not let `ClearToZero` vanish between policy publication and the later mutation guard.
- Do not publish a fallback decision through a second helper that bypasses the owner-private snapshot.
- Do not turn the serialization boundary into a background recovery task.

## Allowed Simplifications

- One private mutex or equivalent owner gate is sufficient.
- One private policy snapshot is sufficient.
- No background worker is required.
- No lock-free publication protocol is required.

## Why No Dedicated Async Worker Is Needed

- The row is about publishing one stable boot-policy result, not about creating a second recovery engine.
- Mount/open and sync are ordinary owner consumers that can read the same published snapshot.
- A background repair task would blur the line between boot policy and later recovery, which is explicitly out of scope here.

## Reviewer And Checker Expectations

- Reviewers should confirm that the design remains a publication boundary only.
- Reviewers should confirm that the boot-source decision and the dirty boot intent are published together from the caller's point of view.
- Reviewers should confirm that `ClearToZero` is treated as a pre-mutation requirement, not as a transient sync flag.
- Checkers should reject any design that adds a background boot-recovery loop or a second boot-policy manager.
