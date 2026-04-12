<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Async

## Metadata

- Component ID: `EXR-READ-OPS-25`
- Title: `ExfatInode` Buffered Read Serialization And Repeated-Call Invariants
- Status: `Specified`
- Author: designer
- Date: `2026-04-12`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-READ-OPS-25/20260412-1110-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/00_architect.md`

## Scope

- In scope:
  - Define per-call serialization expectations for buffered regular-file `read_at`.
  - State how mapping, physical byte copying, and zero-fill sequencing interact on one stable inode snapshot.
  - State repeated-call invariants that later page-cache work must preserve.
- Out of scope:
  - A new cache owner or shared read state.
  - A new lock hierarchy across filesystem read and write paths.
  - Writer-side growth, truncate, allocator mutation, or sync ordering.

## Serialization Contract

- Shared boundaries involved:
  - The immutable inode snapshot on `ExfatInode`.
  - The filesystem-owned traversal context consumed by the current mapping helper shape.
  - The caller-owned `VmWriter`.
- Rule 1:
  - One `read_at` call owns its own mapping, copy, and zero-fill sequencing; it must not depend on a mutable read cursor surviving across calls.
- Rule 2:
  - Mapping remains a per-iteration translation step. The read path may call the mapping helper repeatedly, but it must not promote mapping state into a shared cache or service.
- Rule 3:
  - Physical byte copying and zero-fill emission belong to the same call-local read sequence so the returned byte count matches the exact visible byte stream.
- Rule 4:
  - If future page-cache work wants stronger coordination, that coordination belongs to `EXR-PGCACHE-26`, not to this row.

## Repeated-Call Expectations

- Deterministic reads:
  - Repeating `read_at` on the same inode snapshot, offset, and underlying media should return the same bytes and the same byte count.
- EOF stability:
  - Repeating a read that starts at or beyond logical EOF should always return `0`.
- Valid-size stability:
  - Repeating a read that crosses from physically backed data into the valid-size gap should return the same copied prefix and zero-filled suffix.
- Locality of state:
  - The read path must not store hidden progress state in `ExfatInode`, `ExfatFs`, or the mapping helper between calls.

## Forbidden Interleavings

- Do not hold a future page-cache guard while performing raw physical reads for this row.
- Do not share a mutable mapping cursor or partially consumed `PhysicalFileRange` across callers.
- Do not let one call's zero-fill bookkeeping influence another call.
- Do not interleave read-side policy with write-side size mutation or allocator mutation.

## Allowed Simplifications

- Reconstructing traversal context or mapping state per call is acceptable.
- Repeating the per-cluster mapping and copy loop inside one read call is acceptable.
- A call-local scratch zero buffer is acceptable if it stays subordinate to one `read_at` invocation.

## Reviewer/Checker Expectations

- Reviewers should confirm that buffered read remains a call-local `ExfatInode` method with no promoted cache owner.
- Checkers should confirm that repeated reads on the same snapshot return the same byte stream and byte count.
- Checkers should reject any implementation that introduces hidden mutable read progress, cache publication, or write-side coupling.
