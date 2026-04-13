<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Async

## Metadata

- Component ID: `EXR-NAMESPACE-29`
- Title: `ExfatInode` Namespace Mutation Serialization
- Status: `Specified`
- Author: designer
- Date: 2026-04-13
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-NAMESPACE-29/20260413-1307-designer-repair-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/00_architect.md`

## Scope

- In scope:
  - State the serialization boundary for namespace mutation inside `ExfatInode`.
  - Define what later callers may observe before and after `create`, `unlink`, `mkdir`, `rmdir`, or `rename` completes.
  - Explain why this component does not need a dedicated async protocol or background worker.
  - Keep the `EXR-CHARSET-32` to `EXR-UPCASE-20` handoff synchronous inside namespace preflight.
- Out of scope:
  - A background namespace queue.
  - Deferred publication of child handles.
  - A rename coordinator that lives outside `ExfatInode`.
  - Sync ordering or writeback policy.

## Serialization Contract

- Shared boundaries involved:
  - The owning `ExfatInode` instance.
  - The validated converted-name handoff from `EXR-CHARSET-32`.
  - The filesystem-owned canonicalization and opened-inode publication state reached through `ExfatFs`.
  - The committed allocation result supplied by `EXR-ALLOC-27` when a directory write needs growth.
  - The write-side directory mutation boundary inside `DirectoryEngine`.
- Rule 1:
  - One namespace request owns its own preflight, directory mutation, and publication handoff.
  - Any temporary helper state remains owner-private and does not outlive the namespace call.
- Rule 2:
  - Namespace preflight consumes a validated converted-name value first, then calls `EXR-UPCASE-20` on the converted UTF-16 units.
  - The namespace path does not replay charset conversion or reparse raw `&str`.
  - The namespace path consumes a write boundary and an allocation result only when growth is already decided.
- Rule 3:
  - Later callers should observe either the old namespace state or the fully applied new namespace state.
  - No half-applied create, unlink, mkdir, rmdir, or rename should be published as usable directory state.
- Rule 4:
  - If a caller needs deferred writeback or persistence ordering, that belongs to `EXR-SYNC-31`, not to this row.

## Why No Dedicated Async Artifact Is Needed

- Namespace mutation is a synchronous filesystem-owner concern.
- The component does not introduce background publication, deferred cleanup, or cross-thread reservation visibility.
- The only handoffs this row needs are the validated converted-name value from `EXR-CHARSET-32`, the fold/hash service from `EXR-UPCASE-20`, the committed allocation result from `EXR-ALLOC-27`, the `DirectoryEngine` write boundary, and the canonical child publication boundary in `ExfatFs`.
- That means the core design already covers the necessary serialization discipline, so a separate async protocol would add surface without adding a distinct owner boundary.

## Forbidden Interleavings

- Do not let a later caller observe a partially written directory record as a live namespace entry.
- Do not split source removal and destination publication in `rename` into independently visible phases.
- Do not add a background mutation queue that can race with the owning filesystem state.
- Do not turn namespace mutation into a second allocation pipeline.
- Do not let namespace preflight reopen charset parsing after `EXR-CHARSET-32` has already validated the input.

## Allowed Simplifications

- One `ExfatFs` owner-local serialization boundary is sufficient.
- A temporary owner-private preflight helper is acceptable.
- A committed allocation result may be consumed synchronously when directory growth is required.

## Reviewer And Checker Expectations

- Reviewers should confirm that namespace mutation stays synchronous and owner-local.
- Reviewers should confirm that no background worker, deferred publish queue, or reservation lease appears here.
- Reviewers should confirm that `EXR-CHARSET-32` feeds namespace preflight exactly once and that `EXR-UPCASE-20` remains the only fold/hash owner.
- Checkers should treat any attempt to add an async protocol as boundary drift, not as a refinement of this component.

