<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Async

## Metadata

- Component ID: `EXR-DIR-OPS-23`
- Title: `ExfatInode` Directory-Op Serialization And Repeated-Call Invariants
- Status: `Specified`
- Author: designer
- Date: `2026-04-10`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-OPS-23/20260410-1545-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/00_architect.md`

## Scope

- In scope:
  - Define the per-call serialization expectations for `lookup` and `readdir_at`.
  - State how directory scanning, canonicalization, and child-handle publication interact without inventing a new concurrency owner.
  - State the repeated-call invariants that later checker work should preserve.
- Out of scope:
  - A new lock hierarchy for namespace mutation or allocator policy.
  - Long-lived shared directory iteration state.
  - Background caches or async workers.

## Serialization Contract

- Shared boundaries involved:
  - `DirectoryEngine` per-call scan state.
  - `ExfatFs` canonicalization state.
  - `ExfatFs` opened-inode publication state.
- Rule 1:
  - Each `lookup` or `readdir_at` call owns its local record-stream walk; no call should depend on a shared mutable `DirectoryEngine` instance surviving across calls.
- Rule 2:
  - Name folding and hashing may read filesystem-owned upcase state, but they must not hold that state in a way that overlaps with blocking directory I/O.
- Rule 3:
  - Child publication or reuse for a matched lookup may enter the filesystem-owned opened-inode boundary only after the directory record match has been identified.
- Rule 4:
  - `readdir_at` should expose a stable logical continuation token even if the implementation replays the scan from the beginning on each call.

## Repeated-Call Expectations

- Lookup:
  - Repeating `lookup` for the same present name should return the canonical child handle for the same on-disk record location.
  - Repeating `lookup` for a missing name should remain read-only and should not create placeholder state.
- Readdir:
  - Repeating `readdir_at` with the same starting offset over the same directory snapshot should emit the same logical sequence.
  - Repeating `readdir_at` with the returned next offset should continue after the last emitted entry, not restart from the beginning of visible enumeration.

## Forbidden Interleavings

- Do not hold filesystem-owned publication state while scanning directory records from disk.
- Do not let two concurrent lookups publish two different child handles for the same matched record location.
- Do not let `readdir_at` offset handling depend on hidden mutable scanner state shared across callers.
- Do not let directory-op helpers become an informal second cache or second canonicalization owner.

## Allowed Simplifications

- A fresh `DirectoryEngine` per call is acceptable.
- Replay-based offset handling for `readdir_at` is acceptable if the visible progression remains stable and read-only.
- The existing filesystem-owned publication boundary is sufficient for matched-child reuse.

## Reviewer/Checker Expectations

- Reviewers should confirm that any helper remains local to `ExfatInode` and does not grow into a concurrency owner.
- Checkers should confirm that repeated lookup and resumed readdir behavior are stable across calls.
- Checkers should reject any implementation that performs directory I/O while holding the child-publication boundary.
