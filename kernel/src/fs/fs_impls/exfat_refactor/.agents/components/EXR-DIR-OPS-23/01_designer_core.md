<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-DIR-OPS-23`
- Title: `ExfatInode` Read-Only Directory Operations
- Status: `Specified`
- Author: designer
- Date: `2026-04-11`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-OPS-23/20260411-1622-designer-repair-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/00_architect.md`

## Scope

- In scope:
  - Define `ExfatInode::lookup` and `ExfatInode::readdir_at` as the read-only VFS directory surface for published exFAT directory inodes.
  - Consume a filesystem-owned directory-stream bridge that starts a fresh `DirectoryEngine` for the current inode chain without exposing raw `ExfatFs` fields to `inode.rs`.
  - Consume `UpcaseTable` as the filesystem-owned canonical name-folding and name-hash service.
  - Consume a filesystem-owned child-publication bridge that resolves or reuses the canonical opened inode from trusted record-location facts.
  - Define the narrow owner-private helper shape allowed inside `ExfatInode` so creator work does not invent a separate lookup service.
  - If `DirectoryRecord::File` needs a richer projection to preserve trusted location facts, keep that projection owner-internal to the directory stream and justify it only as consumed output for later `lookup` and `readdir_at` consumers.
  - Keep checker obligations focused on directory-only read paths and repeated-call stability.
- Out of scope:
  - Mount/open sequencing and root publication, which are prerequisites already owned by `EXR-FS-OPEN-22`.
  - Namespace mutation, write-side directory entry updates, allocator policy, and bitmap mutation.
  - Regular-file data-path behavior, page-cache integration, and sync ordering.
  - Any VFS-facing owner other than `ExfatInode`, including a lookup service, scanner shell, mutation shell, or raw `ExfatFs` field accessor.

## Module Specification

- Dependencies:
  - `EXR-INODE-CORE-17` for the stable `ExfatInode` owner and filesystem back-reference.
  - `EXR-DIR-ENGINE-19` for read-only directory record streaming.
  - `EXR-UPCASE-20` for canonical UTF-16 folding and exFAT name hashing.
  - `EXR-FS-OPEN-22` for ready-root publication and post-open filesystem state.
  - The VFS `Inode` directory-method contract.
- Interfaces provided:
  - `ExfatInode::lookup` for name-sensitive read-only child resolution on directory inodes.
  - `ExfatInode::readdir_at` for read-only directory entry enumeration.
  - Owner-private directory-only helpers inside `inode.rs`, if needed, provided they stay subordinate to `ExfatInode`.
- Files or modules touched:
  - Primary landing: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - Likely creator collision point: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs` for the directory-stream bridge, the `InodeKey` derivation boundary, and canonical child publication.
  - Likely creator collision point: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs` if the file-record projection is widened to carry trusted location facts for `lookup`.
  - Trusted location facts to surface alongside a file record: the parent inode identity, the directory byte offset of the record’s primary file dentry, and the primary-entry ordinal within that record.
- Hidden implementation details:
  - Whether `lookup` and `readdir_at` share one or more owner-private scan helpers.
  - Whether the directory scan is expressed as a simple streaming loop or a small local projection helper.
  - Whether readdir offset replay is implemented by rescanning from the beginning or another owner-local read-only strategy, so long as the externally visible offset progression stays stable.

## Functional Specification

### Directory Eligibility

- Preconditions:
  - The current inode is a directory inode already published by the mount/open path or later directory lookup.
- Actions:
  - Treat `lookup` and `readdir_at` as meaningful only for directory inodes.
  - For non-directory inodes, preserve the VFS-visible directory rejection behavior instead of inventing partial semantics.
- Postconditions:
  - This component remains a directory-only read surface on `ExfatInode`.

### Operation

- Name: `ExfatInode::lookup`
- Inputs:
  - A single child name from the VFS layer.
- Preconditions:
  - The inode is a directory.
  - The owning filesystem already has a published root and installed canonicalization prerequisites from mount/open.
- Actions:
  - Obtain the owning filesystem through the existing weak back-reference.
  - Ask `ExfatFs` for a fresh read-only `DirectoryEngine` over the current inode’s validated directory-chain snapshot; do not reach into `block_device`, `super_block`, or opened-inode state directly from `inode.rs`.
  - Use `DirectoryEngine` to stream the directory record set for the current inode’s directory chain.
  - For each file record, derive the exFAT name-comparison inputs from the validated record.
  - Use the accepted `UpcaseTable` owner methods to fold and hash names for comparison; do not reimplement canonicalization locally.
  - On a match, derive the validated `InodeKey` from the record’s trusted parent and primary-entry location facts, then resolve or publish the child inode through the filesystem-owned opened-inode boundary so repeated lookup returns the canonical child handle.
  - Return a missing-entry error when no record matches.
- Outputs:
  - The canonical looked-up child inode handle on success.
  - A read-only lookup miss when the entry does not exist.
- Postconditions:
  - Lookup remains read-only.
  - Name-sensitive matching depends on the installed `UpcaseTable`.
  - Child-handle reuse remains owned by `ExfatFs`, not by `ExfatInode`.
  - The location facts used to build `InodeKey` come from the validated directory record projection, not mutable inode metadata.

### Operation

- Name: `ExfatInode::readdir_at`
- Inputs:
  - A caller-provided offset and a VFS dirent visitor.
- Preconditions:
  - The inode is a directory.
- Actions:
  - Drive a read-only directory scan from the current inode’s directory chain using the filesystem-owned directory-stream bridge and `DirectoryEngine`.
  - Project validated file records into VFS-visible directory entries without mutating directory state.
  - If the file-record projection carries trusted location facts for `lookup`, keep them owner-internal and do not expose them as user-visible dirent data.
  - Treat the caller-provided offset as a stable logical enumeration position over emitted entries, not as permission to expose raw on-disk mutation details.
  - Resume emission from the requested logical position and return the next logical position after the last emitted entry.
  - Do not expose singleton system-entry records such as raw bitmap or upcase metadata as normal user-visible children.
- Outputs:
  - The next logical offset after the entries accepted by the visitor.
- Postconditions:
  - Repeated `readdir_at` calls with the returned offset continue the same read-only enumeration order.
  - Enumeration remains a projection of validated file records only.

### Allowed Helper Shape

- Owner-private helpers may:
  - build a directory-chain scan input from the current inode,
  - compare a candidate record name against the caller name using filesystem-owned canonicalization,
  - derive `InodeKey` from trusted record-location facts and materialize or reuse a child inode through `ExfatFs`.
- Owner-private helpers must not:
  - become a standalone lookup service,
  - hold long-lived scan state outside one `lookup` or `readdir_at` call,
  - absorb mount/open sequencing, mutation, or allocator policy.

## Invariants

- `lookup` and `readdir_at` live on `ExfatInode`, not on `DirectoryEngine` or another helper owner.
- `DirectoryEngine` remains the only record-stream owner and is consumed read-only through `ExfatFs`.
- `UpcaseTable` remains the only canonicalization source for name-sensitive lookup.
- Opened-inode reuse remains owned by `ExfatFs`.
- `lookup` does not mutate directory entries, bitmap state, or allocator state.
- `readdir_at` emits a stable logical enumeration order over validated file records only.
- Root construction and publication are prerequisites, not behavior owned here.
- The trusted location facts required by `InodeKey` come from the directory-record projection, not from inode-local metadata recovery.

## Concurrency Specification

- Shared state:
  - The directory inode snapshot owned by `ExfatInode`.
  - Filesystem-owned canonicalization and opened-inode reuse state reached through `ExfatFs`.
- Lock ordering:
  - Directory scanning and record decoding must happen outside any critical section that serializes opened-inode publication.
  - If lookup needs canonical child publication, derive `InodeKey` from the validated record facts first, then acquire the filesystem-owned publication boundary only around lookup, reuse, or insert of the resolved child handle.
  - Do not hold the opened-inode publication boundary while driving directory I/O through `DirectoryEngine`.
- Atomicity requirements:
  - A successful lookup must return one canonical child inode handle for a matched record.
  - Repeated readdir calls with the returned offset must observe a self-consistent logical progression for the same underlying directory snapshot.
- Forbidden interleavings:
  - Do not perform blocking directory scanning while holding cache-publication state.
  - Do not let `lookup` manufacture duplicate child inode handles for one on-disk record location.
  - Do not let `readdir_at` smuggle mutation-visible state changes into the offset contract.
- Allowed simplifications:
  - Per-call rescanning is acceptable for this read-only unit if it keeps the owner boundary simple and the visible offset progression stable.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Implement directory-only `lookup` and `readdir_at` on `ExfatInode` in `inode.rs`.
  - Use the filesystem-owned directory-stream bridge to obtain a fresh `DirectoryEngine` and `UpcaseTable` to canonicalize names.
  - Reuse child handles through the filesystem-owned child-publication bridge rather than introducing inode-local cache ownership.
  - Derive `InodeKey` from trusted directory-record location facts, not from mutable inode metadata.
  - Keep any helper surface owner-private to `ExfatInode`.
  - Preserve non-directory rejection behavior outside directory inodes.
- Explicit non-goals:
  - No create, unlink, mkdir, rmdir, rename, or link behavior.
  - No write-side directory updates.
  - No mount/open or root publication work.
  - No file read/write or mapping behavior.
  - No raw `ExfatFs` field accessors or standalone lookup service.

### Serial Checker Pass

- Required checker-owned tests:
  - A lookup regression that confirms case-equivalent names resolve through the installed `UpcaseTable` behavior, derive `InodeKey` from trusted record-location facts, and reuse one canonical child handle.
  - A lookup-miss regression that confirms absent names fail without mutating owner state.
  - A readdir regression that confirms validated file records are emitted in stable order, any trusted location facts stay owner-internal, and system entries are not exposed as user-visible children.
  - A continuation regression that confirms `readdir_at` returns a stable next offset and can resume enumeration from that value.
- Observable properties that must pass before leaving the serial loop:
  - Directory ops remain read-only.
  - Name-sensitive lookup depends on filesystem-owned canonicalization rather than local ad hoc matching.
  - Child-handle reuse remains filesystem-owned.
  - Trusted record-location facts are consumed only for `InodeKey` derivation and are not promoted into VFS-visible dirents.

### Concurrency Creator Pass

- Required implementation obligations:
  - No dedicated concurrency implementation beyond the per-call scan plus filesystem-owned publication ordering described above.
- Explicit non-goals:
  - No long-lived shared scanner state.
  - No directory-op-specific lock hierarchy.
  - No background directory cache.
  - No second publication boundary beyond the filesystem-owned child-publication bridge.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - The component remains a read-only inode-method boundary with no extra concurrency machinery.

## Acceptance Notes

- Reviewers should confirm that `lookup` and `readdir_at` remain on `ExfatInode` and do not become a separate service boundary.
- Reviewers should confirm that the repaired spec names the two `ExfatFs` handshakes explicitly: a directory-stream bridge and a canonical-child publication bridge.
- Reviewers should confirm that trusted record-location facts are surfaced only as consumed-owner input for `InodeKey` and not as a new public directory service.
- Reviewers should reject any attempt to fold namespace mutation or mount/open work into this row.
- Reviewers should confirm that `UpcaseTable` and opened-inode reuse are consumed owners rather than reimplemented behaviors.
- Creator slices should be treated as shared-file work across `inode.rs`, `fs.rs`, and `directory.rs`, not as fake file-parallel lanes.
