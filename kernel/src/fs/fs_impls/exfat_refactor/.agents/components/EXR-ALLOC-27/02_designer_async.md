<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Async

## Metadata

- Component ID: `EXR-ALLOC-27`
- Title: `ExfatFs` Allocation Serialization And Reservation Visibility
- Status: `Specified`
- Author: designer
- Date: `2026-04-12`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-ALLOC-27/20260412-1202-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/00_architect.md`

## Scope

- In scope:
  - Define the per-call serialization contract for allocation search, reservation intent, and bitmap/FAT commit.
  - State what later owners may observe and when they may observe it.
  - Explain why this component does not need a dedicated async protocol or background task.
- Out of scope:
  - A background allocator worker.
  - Deferred publication of reservations.
  - Cross-call reservation leases.
  - Any sync-order policy for later writeback or truncate owners.

## Serialization Contract

- Shared boundaries involved:
  - The published allocation bitmap owned by `ExfatFs`.
  - The allocator's transient reservation state.
  - The stable allocation result consumed by later namespace and write owners.
- Rule 1:
  - One allocation request owns its own search, reservation, and commit sequence.
  - Reservation intent must not survive as shared state once the call returns.
- Rule 2:
  - Search is read-only over the published bitmap snapshot.
  - Commit is the only phase allowed to mutate bitmap or FAT state.
- Rule 3:
  - The stable result may be published only after both the bitmap and FAT changes succeed.
  - Later owners must never observe the reservation before the commit completes.
- Rule 4:
  - If a later owner needs deferred work or writeback coordination, that belongs to a different component, not to `EXR-ALLOC-27`.

## Repeated-Call Expectations

- Deterministic search:
  - Repeating the same allocation request against the same published bitmap snapshot should produce the same candidate choice until the bitmap changes.
- Reservation visibility:
  - Repeating a request must not expose a half-committed reservation from an earlier failure.
- Result stability:
  - Once a reservation is committed, later owners should receive the same stable result shape for the same allocation facts.
- Locality of state:
  - The allocator must not store hidden cross-call progress outside the owner boundary unless it is explicitly a private search hint.

## Forbidden Interleavings

- Do not let a later namespace or write owner consume the reservation before the commit finishes.
- Do not split bitmap mutation from FAT mutation into independently visible steps.
- Do not add an async retry loop or background task that can race with later owners.
- Do not let file-growth policy or truncate ordering feed back into allocation visibility.

## Allowed Simplifications

- Per-call serialization under the existing `ExfatFs` owner boundary is sufficient.
- A private search hint may be updated synchronously after each allocation request.
- A temporary owner-local reservation object may be used within one call.

## Why No Dedicated Async Artifact Is Needed

- Allocation under `EXR-ALLOC-27` is a synchronous filesystem-owner concern.
- The component does not introduce background publication, deferred cleanup, or cross-thread coordination.
- The only visibility boundary is the point where the owner publishes the committed allocation result.
- That means the core design already covers the necessary concurrency discipline, and no separate async protocol is required.

## Reviewer/Checker Expectations

- Reviewers should confirm that allocation state remains serialized inside `ExfatFs`.
- Reviewers should confirm that later owners see only committed results.
- Checkers should reject any implementation that introduces a background allocator, a reservation lease, or a deferred publish queue.

