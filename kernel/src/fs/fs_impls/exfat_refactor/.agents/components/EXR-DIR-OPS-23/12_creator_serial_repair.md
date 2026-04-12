<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Repair Log

## Metadata

- Component ID: `EXR-DIR-OPS-23`
- Title: `ExfatInode` Read-Only Directory Operations
- Status: `SerialRepaired`
- Author: Codex
- Date: `2026-04-11`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-OPS-23/20260411-1627-creator-repair-packet.md`
- Implemented spec: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md`
- Pass kind: `serial repair`

## Planned File Ownership

- Files edited:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/12_creator_serial_repair.md`
- Files intentionally left untouched:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/03_designer_ktest.md`
  - sibling checker, reviewer, advisor, and handoff artifacts

## Implementation Notes

Repaired the blocked creator path by landing the three missing handshakes named by the designer set.

- `directory.rs` now keeps trusted record-location facts owner-internal to the directory stream via `DirectoryRecordLocation` and `DirectoryFileRecord`.
- Those file-record projections carry only the consumed facts needed by this row:
  - the validated `ExfatDentrySet`,
  - the parent inode identity,
  - the directory byte offset of the primary file dentry,
  - and the primary entry index used for `InodeKey`.
- The projection also derives a stable synthetic inode number from those validated location facts so `readdir_at` can stay a read-only projection instead of publishing child handles for every visible entry.

- `fs.rs` now exposes the two repaired owner-facing bridges callable from `inode.rs`:
  - `ExfatFs::directory_stream(...)` creates a fresh read-only `DirectoryEngine` from inode-owned chain snapshot facts without exposing raw filesystem fields.
  - `ExfatFs::resolve_or_publish_child_inode(...)` derives `InodeKey` from the trusted record projection, reuses an already-opened inode when present, or constructs and publishes one canonical child inode when absent.
- The child-publication bridge keeps directory scanning outside the opened-inode publication boundary by doing reuse lookup, inode construction, and final publication as separate steps.

- `inode.rs` now owns the VFS-facing read-only directory behavior:
  - `lookup` rejects non-directories, folds the caller name through the filesystem-owned upcase service, scans validated file records through a fresh directory stream, and resolves the canonical child inode through the new filesystem-owned publication bridge.
  - `readdir_at` rejects non-directories, rescans from the start on each call, skips raw singleton system entries, emits only validated file records, and keeps the trusted location facts owner-internal.
- Added two owner-private helpers on `ExfatInode` only for this pass:
  - `record_name_matches(...)` for filesystem-owned canonical comparison.
  - `visible_record_name(...)` for UTF-16 to VFS string projection.

## Added Helper Surfaces

- `DirectoryRecordLocation`
  - Final owner: `DirectoryEngine` consumed output only.
  - Purpose: carry the validated location facts that feed `InodeKey` and stable dirent inode numbering.
- `DirectoryFileRecord`
  - Final owner: `DirectoryEngine` consumed output only.
  - Purpose: keep file-record payload plus location facts together for `lookup`, `readdir_at`, and canonical child publication.
- `ExfatFs::directory_stream(...)`
  - Final owner: `ExfatFs`.
  - Purpose: the repaired directory-stream bridge from inode-owned chain snapshot facts.
- `ExfatFs::resolve_or_publish_child_inode(...)`
  - Final owner: `ExfatFs`.
  - Purpose: the repaired canonical-child publication bridge for lookup.
- `ExfatInode::record_name_matches(...)` and `ExfatInode::visible_record_name(...)`
  - Final owner: `ExfatInode`.
  - Purpose: owner-private lookup/readdir helpers only; not a new service boundary.

## Approved Deviations

- Child inode metadata created through the new publication bridge currently keeps root-style zero timestamps because this repair packet did not authorize a broader timestamp-decoding slice.
- The synthetic inode number used by `readdir_at` is derived only from trusted location facts so the enumeration can stay read-only and stable across rescans.

## Optional Self-Checks

- Commands run, if any: read-only inspection only (`sed`, `rg`, `git status`).
- Compile checks run, if any: none; the packet required a command-free creator repair lane.
- Manual reasoning checks:
  - Confirmed `inode.rs` no longer reaches into raw `ExfatFs` fields.
  - Confirmed directory I/O still happens before any child-publication lock acquisition.
  - Confirmed `lookup` consumes filesystem-owned canonicalization and filesystem-owned child reuse.
  - Confirmed `readdir_at` emits only validated file records and keeps trusted location facts internal to the read-only path.

## Remaining Risks

- The repair was not compile-verified in this lane by design.
- Child metadata created through the new publication bridge uses a conservative synthesized mode and zeroed timestamps until a later metadata/time slice owns fuller decoding.
- The repaired `readdir_at` continuation contract was implemented directly from the designer guidance and should be checker-verified against the surrounding VFS handle expectations.
