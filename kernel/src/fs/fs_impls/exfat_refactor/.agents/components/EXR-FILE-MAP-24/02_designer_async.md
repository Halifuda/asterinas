<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Async

## Metadata

- Component ID: `EXR-FILE-MAP-24`
- Title: `ExfatInode` Read-Path Mapping Serialization And Repeated-Call Invariants
- Status: `Specified`
- Author: designer
- Date: `2026-04-11`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FILE-MAP-24/20260411-1613-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/00_architect.md`

## Scope

- In scope:
  - Define the per-call serialization expectations for the owner-private mapping helpers on `ExfatInode`.
  - State how chain traversal, inode-owned size facts, and cluster geometry interact without creating a new concurrency owner.
  - State the repeated-call invariants later reader work must preserve.
- Out of scope:
  - A new lock hierarchy for read-path policy.
  - Long-lived shared mapping state or a chain cursor cache.
  - Page-cache coordination, zero-fill ownership, or data-copy ownership.

## Serialization Contract

- Shared boundaries involved:
  - The inode-owned snapshot fields carried by `ExfatInode`.
  - The filesystem-owned cluster geometry reached through `ExfatFs`.
  - The read-only `ExfatChain` traversal boundary used to find the target cluster.
- Rule 1:
  - Each mapping call owns its own translation work; no helper should require a mutable chain cursor to survive across calls.
- Rule 2:
  - The helper may read inode-owned size facts and filesystem geometry together, but it must not retain those values as a hidden mutable cache.
- Rule 3:
  - If a later read owner serializes mapping and byte-copying, that serialization belongs to the caller, not to the helper surface.
- Rule 4:
  - A fresh `ExfatChain` reconstruction per call is acceptable when it keeps the inode boundary simple and deterministic.

## Repeated-Call Expectations

- Offset translation:
  - Repeating the same logical-offset translation on the same inode snapshot should return the same cluster position and in-cluster byte offset.
  - Repeating the same translation after the inode snapshot changes is outside this unit and belongs to later owner invalidation logic.
- Span derivation:
  - Repeating the same span request on the same snapshot should return the same physically mappable byte count.
  - The helper must not depend on hidden scanner state, so the result should not drift across calls with the same inputs.
- Read-side caller behavior:
  - Later read-side callers may call the mapping helpers back-to-back on one stable snapshot before deciding how to copy, zero-fill, or stop.
  - Those callers should treat the helper output as a pure read-side translation result, not as a durable object that needs lifecycle management.

## Forbidden Interleavings

- Do not hold a page-cache lock while walking the chain.
- Do not hold a write-side mutation guard while deriving the mapped span.
- Do not share a mutable offset cursor across callers.
- Do not let mapping helpers become an informal cache owner or a second filesystem owner.
- Do not fold EOF policy or zero-fill policy into the helper boundary.

## Allowed Simplifications

- Per-call chain reconstruction is acceptable.
- Caller-local serialization around the helper is acceptable if a later read owner wants to keep translation and copy ordered.
- The helper can remain a small pure translation layer even if later buffered-read code invokes it many times per file read.

## Reviewer/Checker Expectations

- Reviewers should confirm that the helpers remain local to `ExfatInode` and do not grow into a separate read service.
- Checkers should confirm that repeated calls with the same snapshot and logical inputs produce the same outputs.
- Checkers should reject any implementation that makes the mapping layer dependent on hidden mutable cursor state or page-cache coordination.
