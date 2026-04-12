<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Async

## Metadata

- Component ID: `EXR-DENTRY-WRITE-28`
- Title: `DirectoryEngine` Write-Side Mutation Serialization
- Status: `Specified`
- Author: designer
- Date: 2026-04-12
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DENTRY-WRITE-28/20260412-2049-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/00_architect.md`

## Scope

- In scope:
  - State the serialization boundary for write-side directory mutation inside `DirectoryEngine`.
  - Define what later callers may observe before and after a directory write completes.
  - Explain why this component does not need a dedicated async protocol.
- Out of scope:
  - A background directory-writer worker.
  - Deferred tombstone publication.
  - A cross-call reservation lease.
  - Sync ordering or writeback policy.

## Serialization Contract

- Shared boundaries involved:
  - The owning `DirectoryEngine` instance inside `ExfatFs`.
  - The validated `ExfatDentrySet` supplied by `EXR-FILESET-04B`.
  - The committed allocation result supplied by `EXR-ALLOC-27` when growth is already decided.
- Rule 1:
  - One write request owns its own placement, overwrite, tombstone, and growth handoff.
  - Any temporary placement state remains owner-private and does not outlive the write call.
- Rule 2:
  - Validation and serialization are not replayed here.
  - The write path consumes validated bytes and a committed growth fact, then places them in the directory.
- Rule 3:
  - Later callers should observe either the old directory content or the fully written new content.
  - No half-placed record should be published as a usable directory entry.
- Rule 4:
  - If the caller needs deferred writeback or namespace coordination, that belongs to another component, not to this row.

## Why No Dedicated Async Artifact Is Needed

- Directory-entry mutation is a synchronous filesystem-owner concern.
- The component does not introduce background publication, deferred cleanup, or cross-thread reservation visibility.
- The only handoff this row needs is the committed allocation result already produced by `EXR-ALLOC-27`.
- That means the core design already covers the necessary serialization discipline, so a separate async protocol would add surface without adding a distinct owner boundary.

## Forbidden Interleavings

- Do not let a later namespace owner observe a partially written directory record.
- Do not split overwrite and tombstoning into independently visible phases.
- Do not add a background write queue that can race with the owning filesystem state.
- Do not turn directory growth into a second allocation pipeline.

## Allowed Simplifications

- One `ExfatFs` owner-local serialization boundary is sufficient.
- A temporary owner-private placement cursor or location fact is acceptable.
- A committed allocation result may be consumed synchronously when growth is required.

## Reviewer And Checker Expectations

- Reviewers should confirm that the write path stays synchronous and owner-local.
- Reviewers should confirm that no background writer, deferred tombstone queue, or reservation lease appears here.
- Checkers should treat any attempt to add an async protocol as a boundary drift, not as a refinement of this component.
