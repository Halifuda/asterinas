<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-BOOT-34`
- Title: `ExfatFs` Boot-Region Fallback And Persistent Boot-Flag Policy
- Status: `Specified`
- Author: designer
- Date: `2026-04-13`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BOOT-34/20260413-1421-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/00_architect.md`

## Scope

- In scope:
  - Define the smallest `ExfatFs`-owned policy surface that chooses whether mount/open trusts the validated primary boot facts or an owner-private backup fallback candidate.
  - Define when `VolumeDirty` becomes persistent boot-region intent owned by `ExfatFs`.
  - Define when `ClearToZero` is remembered as a required pre-mutation clear before later volume changes.
  - Treat `PercentInUse` as an observational input unless a later owner proves a real policy use.
  - Specify how `EXR-FS-OPEN-22` consumes the policy result without absorbing boot-region ownership.
  - Specify how `EXR-SYNC-31` later consumes only the published dirty boot intent, not the selection logic itself.
- Out of scope:
  - Primary boot parsing, validation, and checksum verification.
  - Backup boot parsing or checksum verification as a separate owner.
  - Volume-label control.
  - Direct I/O.
  - Trim/discard.
  - Forced shutdown.
  - FAT-attribute ioctls.
  - Any background recovery worker or public boot-policy API.

## Module Specification

- Dependencies:
  - `EXR-BOOT-01` for validated primary boot facts.
  - `EXR-FS-OPEN-22` for mount/open sequencing and root publication.
  - `EXR-SYNC-31` for filesystem-wide sync ordering.
  - The existing `ExfatFs` owner boundary in `fs.rs`.
- Interfaces provided:
  - A private `ExfatFs` boot-region policy result.
  - A private mount/open selector that returns the trusted boot source.
  - A private persistent boot-intent snapshot that later sync code can consume.
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- Hidden implementation details:
  - Whether the policy result is a private struct, enum, or tuple, so long as it stays owner-local to `ExfatFs`.
  - Whether the fallback candidate is threaded through the owner boundary as an optional already-validated fact bundle or as an equivalent private helper input.
  - The exact private field names used to remember persistent boot intent.

## Functional Specification

### Operation

- Name: Boot-region policy selection
- Inputs:
  - Validated primary boot facts from `EXR-BOOT-01`.
  - Optional owner-private backup candidate facts if a later helper has already validated them.
  - The current `ExfatFs` owner-private mount state.
- Preconditions:
  - This row does not parse or checksum either boot region itself.
  - Any fallback candidate already exists as validated facts before this policy helper sees it.
- Actions:
  - Choose the boot-region source that mount/open should trust.
  - Prefer the validated primary facts unless the owner-private fallback candidate is the selected trusted source.
  - Record the persistent `VolumeDirty` and `ClearToZero` boot-region intent bits in owner-private state.
  - Treat `PercentInUse` as an observation unless the caller has already proven a bounded policy use.
  - Do not widen the row into recovery, bitmap ownership, or free-space accounting.
- Outputs:
  - A private boot-region policy result that mount/open can consume.
- Postconditions:
  - Later mount/open code can reuse the same trusted boot source without re-deciding the policy.
  - Later sync code can consume the dirty boot intent without re-running the selection logic.

### Operation

- Name: Mount/open policy consumption
- Inputs:
  - The private boot-region policy result.
- Preconditions:
  - `EXR-FS-OPEN-22` remains the owner of mount/open sequencing.
- Actions:
  - Let mount/open trust the selected boot source and continue publication through `ExfatFs`.
  - Keep the root-publication handoff and opened-inode reuse owned by `EXR-FS-OPEN-22`.
  - Do not allow mount/open to become a second boot-policy owner.
- Outputs:
  - A mount-ready `ExfatFs` state that still remembers the persistent boot intent.
- Postconditions:
  - The mount/open path uses the policy result as input, not as a new decision point.

### Operation

- Name: Sync-side dirty-boot intent consumption
- Inputs:
  - The published persistent boot intent owned by `ExfatFs`.
- Preconditions:
  - `EXR-SYNC-31` remains the only filesystem-wide flush-ordering owner.
- Actions:
  - Let later sync code observe only the published dirty boot intent.
  - Keep the boot-source decision itself out of sync ordering.
  - Do not let sync own fallback selection, boot parsing, or checksum validation.
- Outputs:
  - A dirty boot-region snapshot suitable for later writeback ordering.
- Postconditions:
  - Sync can drain the already-published boot intent without absorbing policy logic.

## Invariants

- `ExfatFs` is the only owner of the boot-region policy result.
- The policy helper does not parse or validate boot sectors.
- `EXR-FS-OPEN-22` consumes the trusted boot source but does not own the decision.
- `EXR-SYNC-31` consumes only the dirty boot intent, not the decision logic.
- `VolumeDirty` and `ClearToZero` are persistent boot-region outputs, not generic inode metadata.
- `ClearToZero` remains a pre-mutation requirement, not a mount-time policy knob.
- `PercentInUse` remains observational unless a later owner proves a bounded use.
- No public boot-policy API, background recovery worker, or separate sync manager is introduced here.

## Concurrency Specification

- Shared state:
  - The private boot-region policy result.
  - The private persistent boot-intent snapshot.
  - The mount/open consumer state inside `ExfatFs`.
- Lock ordering:
  - Publish the boot-region policy before exposing the ready mount/open state.
  - Publish dirty boot intent before any later sync consumer can drain it.
  - Keep boot-policy publication serialized through the same owner boundary that mount/open and sync later use.
- Atomicity requirements:
  - Callers must see either the pre-policy state or the fully published boot-policy state.
  - The trusted boot source and the persistent boot intent must change together when the policy result changes.
- Forbidden interleavings:
  - Do not let mount/open see a half-published fallback decision.
  - Do not let sync clear `VolumeDirty` before the policy result has been published.
  - Do not let `ClearToZero` disappear between selection and the later mutation guard.
  - Do not introduce a background recovery path that races with mount/open.
- Allowed simplifications:
  - One private owner gate in `fs.rs` is sufficient.
  - One private policy snapshot is sufficient.
  - No background worker or deferred recovery queue is required.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add owner-private boot-region policy helpers in `fs.rs`.
  - Record the trusted boot source and persistent boot intent in `ExfatFs`-owned state.
  - Expose the published dirty boot intent to the later sync owner without exposing the decision logic.
  - Keep the policy helper narrow enough that `EXR-FS-OPEN-22` and `EXR-SYNC-31` remain the only consumers.
- Explicit non-goals:
  - No second boot parser.
  - No background recovery worker.
  - No public boot-policy surface.
  - No direct I/O, label control, trim/discard, forced shutdown, or FAT-attribute ioctl work.

### Serial Checker Pass

- Required checker-owned tests:
  - A mount/open regression that proves the selected boot source is stable once the policy result has been published.
  - A persistent-intent regression that proves `VolumeDirty` survives as boot-region writeback intent.
  - A pre-mutation regression that proves `ClearToZero` remains a required clear-before-mutation bit.
  - An observation regression that proves changing only `PercentInUse` does not change the trusted boot source or dirty-intent publication.
  - A sync-handoff regression that proves later sync code sees the published dirty boot intent and not the selection logic itself.
- Observable properties that must pass before leaving the serial loop:
  - The owner boundary can publish one stable boot-policy result.
  - The mount/open consumer and sync consumer remain separate from the policy decision itself.
  - The row does not widen into backup parsing, recovery, or free-space accounting.

### Concurrency Creator Pass

- Required implementation obligations:
  - Keep policy publication serialized through the `ExfatFs` owner boundary.
  - Ensure the trusted boot source and persistent boot intent are published atomically from the caller's point of view.
- Explicit non-goals:
  - No lock-free policy state.
  - No async recovery task.
  - No second writer for the same boot-intent snapshot.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - A regression that exercises concurrent mount/open and sync entry points against the same filesystem instance and confirms they observe one boot-policy result.
- Observable properties that must pass before leaving the concurrency loop:
  - The boot-policy result is not published twice.
  - Mount/open does not race with sync into two different trusted sources.
  - The persistent boot intent remains owner-private and linearizable.

## Acceptance Notes

- Reviewers should confirm that this row stays above validated primary boot facts and does not become a second parser.
- Reviewers should confirm that `EXR-FS-OPEN-22` consumes the trusted boot source instead of re-owning it.
- Reviewers should confirm that `EXR-SYNC-31` consumes only the dirty boot intent and not the policy decision logic.
- Reviewers should confirm that `PercentInUse` does not become a hidden free-space owner.
- Reviewers should reject any attempt to fold volume-label control, direct I/O, trim/discard, forced shutdown, or admin ioctls into this policy boundary.
