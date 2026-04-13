<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-NAMESPACE-29`
- Title: `ExfatInode` Namespace Mutation
- Status: `Specified`
- Author: designer
- Date: 2026-04-13
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-NAMESPACE-29/20260413-1307-designer-repair-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/00_architect.md`

## Scope

- In scope:
  - Define `ExfatInode::create`, `unlink`, `mkdir`, `rmdir`, and `rename` as the inode-owned namespace mutation surface for exFAT.
  - Make the namespace preflight handoff explicit: `EXR-CHARSET-32` supplies a validated converted-name value first, `EXR-NAMESPACE-29` consumes that value next, and `EXR-UPCASE-20` folds and hashes the converted UTF-16 units after that.
  - Consume `DirectoryEngine` write primitives as the directory-entry mutation service.
  - Consume committed allocation results only when directory growth is already required.
  - Consume the existing `ExfatFs` opened-inode publication boundary for canonical child-handle reuse.
  - Define the narrow owner-private helper shape allowed inside `inode.rs` so creator work does not invent a standalone namespace manager.
- Out of scope:
  - Read-only directory lookup and enumeration, which remain owned by `EXR-DIR-OPS-23`.
  - Allocation search, reservation intent, and commit coordination, which remain owned by `EXR-ALLOC-27`.
  - Slot placement, overwrite, tombstoning, and directory growth mechanics, which remain owned by `EXR-DENTRY-WRITE-28`.
  - Sync ordering, durability policy, and writeback sequencing, which remain downstream in `EXR-SYNC-31`.
  - Any namespace service layer, background coordinator, or helper owner that would sit between `ExfatInode` and the existing services.
  - Volume-label control, which remains out of this row.

## Module Specification

- Dependencies:
  - `EXR-INODE-CORE-17` for the stable `ExfatInode` carrier and filesystem back-reference.
  - `EXR-CHARSET-32` for the validated converted-name value consumed during namespace preflight.
  - `EXR-UPCASE-20` for fold and hash behavior over the converted UTF-16 units.
  - `EXR-DIR-OPS-23` for the read-side directory surface used during mutation preflight.
  - `EXR-DENTRY-WRITE-28` for the write-side `DirectoryEngine` mutation boundary.
  - `EXR-ALLOC-27` for committed allocation results consumed during directory growth.
  - The existing opened-inode publication boundary under `ExfatFs`.
  - The VFS `Inode` mutation contract.
- Interfaces provided:
  - `ExfatInode::create`
  - `ExfatInode::unlink`
  - `ExfatInode::mkdir`
  - `ExfatInode::rmdir`
  - `ExfatInode::rename`
  - Owner-private namespace-preflight helpers inside `inode.rs`, if needed, provided they remain subordinate to `ExfatInode`.
- Files or modules touched:
  - Primary landing: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - Likely collision point: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs` for the consumed converted-name service and canonical child publication.
  - Likely collision point: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs` for the consumed write-side mutation helper surface.
- Hidden implementation details:
  - Whether `create`, `unlink`, `mkdir`, `rmdir`, and `rename` share one or more owner-private namespace helpers.
  - Whether mutation preflight and mutation execution are separated into small local helper steps or kept in one linear owner method body.
  - Whether `rename` reuses the same helper path as `create` and `unlink` for converted-name consumption and publication, so long as the helper remains owner-private.

## Functional Specification

### Namespace Eligibility

- Preconditions:
  - The current inode is a directory inode already published by mount/open or prior directory lookup.
- Actions:
  - Treat `create`, `unlink`, `mkdir`, `rmdir`, and `rename` as meaningful only for directory inodes.
  - Preserve the existing VFS-visible rejection behavior for non-directory inodes rather than inventing partial semantics.
- Postconditions:
  - This component remains the inode-owned namespace surface.

### Namespace Preflight Handoff

- Inputs:
  - A validated converted-name value from `EXR-CHARSET-32`.
- Preconditions:
  - The caller has already crossed the `ExfatFs` conversion boundary and is not asking `EXR-NAMESPACE-29` to parse raw `&str` again.
- Actions:
  - Consume the validated converted-name value as the first namespace-preflight input.
  - Pass the converted UTF-16 units to `EXR-UPCASE-20` for fold and hash preparation.
  - Keep raw string parsing, UTF-8 validation, and UTF-16 construction outside this row.
- Outputs:
  - A namespace-preflight context that is already operating on validated converted UTF-16 units.
- Postconditions:
  - `EXR-NAMESPACE-29` no longer owns charset conversion.
  - `EXR-UPCASE-20` remains the only owner of fold and hash behavior.

### Operation

- Name: `ExfatInode::create`
- Inputs:
  - A validated converted child-name value, child inode type, and mode.
- Preconditions:
  - The parent inode is a directory.
  - The caller has already obtained the converted-name value from `EXR-CHARSET-32`.
- Actions:
  - Build the namespace-preflight context from the converted child-name value.
  - Fold and hash the converted UTF-16 units through `EXR-UPCASE-20`.
  - Resolve the current directory through the accepted read-side directory surface.
  - Ask `DirectoryEngine` to place the validated directory record set using the consumed write-side mutation boundary.
  - Consume a committed allocation result only if the directory must grow.
  - Publish the resulting child inode through the existing `ExfatFs` opened-inode boundary so repeated creation or lookup uses the canonical handle.
- Outputs:
  - The canonical child inode handle on success.
- Postconditions:
  - Namespace mutation remains inode-owned.
  - Child-handle reuse remains owned by `ExfatFs`.
  - Allocation search and reservation remain outside this component.

### Operation

- Name: `ExfatInode::unlink`
- Inputs:
  - A validated converted child-name value.
- Preconditions:
  - The parent inode is a directory.
- Actions:
  - Consume the converted-name value in namespace preflight.
  - Fold and hash the converted UTF-16 units through `EXR-UPCASE-20`.
  - Resolve the matching namespace entry through the read-side directory surface.
  - Ask `DirectoryEngine` to tombstone or remove the matching validated record through the write boundary.
  - Keep the delete flow inside the inode owner and do not introduce a separate directory-delete service.
- Outputs:
  - Success when the entry is removed.
- Postconditions:
  - The directory no longer exposes the removed child as live namespace state.

### Operation

- Name: `ExfatInode::mkdir`
- Inputs:
  - A validated converted child-name value and mode.
- Preconditions:
  - The parent inode is a directory.
- Actions:
  - Consume the converted-name value in namespace preflight.
  - Fold and hash the converted UTF-16 units through `EXR-UPCASE-20`.
  - Build the child directory namespace using the same inode-owned mutation flow as `create`.
  - Consume a committed allocation result only if the directory write must grow.
  - Publish the resulting child directory handle through the existing `ExfatFs` boundary.
- Outputs:
  - The canonical child directory handle on success.
- Postconditions:
  - Directory creation remains a namespace mutation, not a new owner boundary.

### Operation

- Name: `ExfatInode::rmdir`
- Inputs:
  - A validated converted child-name value.
- Preconditions:
  - The parent inode is a directory.
- Actions:
  - Consume the converted-name value in namespace preflight.
  - Fold and hash the converted UTF-16 units through `EXR-UPCASE-20`.
  - Resolve the matching child directory through the accepted read-side surface.
  - Ask `DirectoryEngine` to remove the validated directory record through the write boundary.
  - Keep removal sequencing inside `ExfatInode`; do not invent a standalone directory-removal manager.
- Outputs:
  - Success when the directory entry is removed.
- Postconditions:
  - The removed directory no longer remains reachable as a live namespace entry.

### Operation

- Name: `ExfatInode::rename`
- Inputs:
  - A validated converted source-name value, a target directory inode, and a validated converted destination-name value.
- Preconditions:
  - The source inode is a directory.
  - The target inode is a directory in the same mounted filesystem.
- Actions:
  - Consume the converted source and destination names in namespace preflight.
  - Pass both converted UTF-16 name values to `EXR-UPCASE-20` for fold/hash preparation.
  - Resolve the source entry through the read-side directory surface.
  - Use the consumed `DirectoryEngine` write boundary to remove or relocate the source record and to publish the destination record.
  - Consume a committed allocation result only if the destination side needs directory growth.
  - Keep the cross-directory coordination inside `ExfatInode` and do not promote rename into a separate service owner.
- Outputs:
  - Success when the source entry is moved or replaced under the destination name.
- Postconditions:
  - Rename remains one inode-owned namespace mutation, not a second helper manager.

### Allowed Helper Shape

- Owner-private helpers may:
  - build a namespace-preflight context from the current inode and the validated converted-name value,
  - route converted UTF-16 units through `EXR-UPCASE-20` for fold/hash preparation,
  - coordinate one mutation call with `DirectoryEngine`,
  - consume a committed allocation result when directory growth is already decided,
  - route child publication through `ExfatFs`.
- Owner-private helpers must not:
  - become a standalone namespace service,
  - absorb allocation search or reservation,
  - hold a long-lived mutation queue or publication cache,
  - absorb sync ordering or durability policy,
  - reopen raw `&str` parsing or charset validation.

## Invariants

- `create`, `unlink`, `mkdir`, `rmdir`, and `rename` live on `ExfatInode`, not on `DirectoryEngine` or a separate namespace owner.
- `EXR-CHARSET-32` owns the `&str` to validated converted-name boundary.
- `EXR-UPCASE-20` remains the only owner of fold and hash behavior over converted UTF-16 units.
- `DirectoryEngine` remains the only owner of directory-entry placement and mutation mechanics.
- `Allocator` remains the only owner of allocation search, reservation intent, and commit.
- Opened-inode reuse remains owned by `ExfatFs`.
- Namespace mutation does not invent a background coordinator or a new service layer.
- Sync ordering remains outside this component and stays in `EXR-SYNC-31`.

## Concurrency Specification

- Shared state:
  - The directory inode snapshot owned by `ExfatInode`.
  - The validated converted-name handoff reached through `EXR-CHARSET-32`.
  - Filesystem-owned canonicalization and opened-inode publication state reached through `ExfatFs`.
  - The write-side directory mutation state owned by `DirectoryEngine`.
- Lock ordering:
  - Consume the validated converted-name value before any publication step that can reuse or insert a child inode handle.
  - Do not hold the opened-inode publication boundary while driving directory I/O through `DirectoryEngine`.
  - If a mutation needs both directory growth and child publication, consume the committed allocation result first, then run the directory write, then publish through `ExfatFs`.
- Atomicity requirements:
  - A successful mutation must present one canonical namespace result for one logical directory operation.
  - Repeated callers must observe either the old directory state or the fully applied new state.
- Forbidden interleavings:
  - Do not let allocation search or reservation leak into namespace mutation.
  - Do not let `rename` expose a partially moved child as a live entry.
  - Do not let mutation preflight become a long-lived mutable coordinator.
- Allowed simplifications:
  - One filesystem-owner serialization boundary is sufficient for the namespace flow.
  - Per-call preflight is acceptable if the externally visible state remains serialized.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Implement the namespace methods on `ExfatInode` in `inode.rs`.
  - Consume the validated converted-name value from `EXR-CHARSET-32` before namespace preflight continues.
  - Use `EXR-UPCASE-20` through `ExfatFs` for canonical UTF-16 fold and hash preparation.
  - Consume `DirectoryEngine` write methods for the actual record mutation.
  - Consume committed allocation results only when a directory write must grow.
  - Reuse the canonical child publication boundary in `ExfatFs`.
  - Keep helper surface owner-private to `ExfatInode`.
- Explicit non-goals:
  - No read-only lookup redesign.
  - No allocation search or reservation logic.
  - No write-side directory slot logic.
  - No sync ordering or durability policy.
  - No standalone namespace manager.
  - No direct raw `&str` parsing in namespace mutation.

### Serial Checker Pass

- Required checker-owned tests:
  - A create or mkdir regression that confirms the new child becomes visible through the inode-owned namespace surface after the validated converted-name handoff and reuses the canonical child publication boundary.
  - An unlink or rmdir regression that confirms the removed entry is no longer visible as live namespace state.
  - A rename regression that confirms the source and destination entries are coordinated through the inode owner and do not require a separate namespace manager.
  - A growth regression that confirms namespace mutation consumes committed allocation results only when the directory write needs more room.
  - A boundary regression that confirms namespace preflight consumes the validated converted-name value instead of reopening raw `&str` parsing.
- Observable properties that must pass before leaving the serial loop:
  - Namespace mutation remains inode-owned.
  - Directory-entry mutation remains inside `DirectoryEngine`.
  - Allocation search and reservation remain outside the namespace path.
  - Child-handle reuse remains filesystem-owned.
  - Charset conversion remains upstream in `EXR-CHARSET-32`.

### Concurrency Creator Pass

- Required implementation obligations:
  - No dedicated concurrency implementation beyond the owner-local serialization boundary described above.
- Explicit non-goals:
  - No background mutation queue.
  - No deferred child publication.
  - No rename coordinator outside `ExfatInode`.
  - No new sync ordering policy.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - The component remains a single owner-local namespace mutation service.
  - The converted-name boundary stays upstream in `EXR-CHARSET-32`.
  - Fold/hash behavior stays owned by `EXR-UPCASE-20`.

## Acceptance Notes

- The reviewer should confirm that the namespace preflight handoff is `EXR-CHARSET-32` -> `EXR-NAMESPACE-29` -> `EXR-UPCASE-20`.
- The reviewer should confirm that the design keeps the validated converted-name boundary separate from directory mutation and from opened-inode publication.
- The reviewer should confirm that volume-label control is not pulled into this row.
- The reviewer should confirm that sync ordering still belongs to `EXR-SYNC-31`.

