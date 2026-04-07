<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: EXR-FS-CORE-16
- Title: ExfatFs Filesystem Owner Boundary
- Status: `Specified`
- Author: designer
- Date: 2026-04-07
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FS-CORE-16/20260407-1035-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/00_architect.md`

## Scope

- In scope:
  - Introduce `ExfatFs` as the stable VFS `FileSystem` carrier and filesystem-wide runtime-state root.
  - Land the `FileSystem` methods that are in scope now: `name()`, `sb()`, and `fs_event_subscriber_stats()`.
  - Keep `root_inode()` as the explicit temporary seam with the architect-approved comment and exit plan to `EXR-FS-OPEN-22`.
  - Keep `sync()` as an explicit placeholder and not a hidden flush-order implementation.
  - Define checker-owned `#[ktest]` coverage for the owner skeleton and the temporary seam.
- Out of scope:
  - Mount/open sequencing, inode ownership, inode cache, directory services, allocation policy, upcase loading, bitmap loading, and any data-path behavior.
  - Any helper type that exists only to hide `root_inode()` or to split off a separate stats shell, mount shell, or sync manager.
  - Real flush ordering, dirty-state traversal, or writeback policy; those belong to `EXR-SYNC-31`.
  - Production-code edits outside the future `fs.rs` owner file and any module declaration needed to wire it in.

## Module Specification

- Dependencies:
  - The validated boot and superblock facts already accepted for the refactor.
  - The VFS `FileSystem` contract and `FsEventSubscriberStats`.
  - The normalized `ExfatSuperBlock` produced by the boot/superblock stage.
- Interfaces provided:
  - `ExfatFs` as the filesystem-wide owner object.
  - The `FileSystem` identity surface for `ExfatFs`: `name()`, `sb()`, `fs_event_subscriber_stats()`, `root_inode()`, and `sync()`.
  - The inherited `FileSystem` defaults for `source()`, `flags()`, and `set_fs_flags()` remain untouched for now.
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs` only if the new owner file needs a module declaration; any such declaration edit must be serialized with sibling `EXR-INODE-CORE-17`.
- Hidden implementation details:
  - The exact private field layout of `ExfatFs`, so long as the owner can answer the stable `FileSystem` surface without recreating state on every call.
  - Whether the owner caches the normalized `SuperBlock` snapshot directly or derives it from stored mount facts, provided the snapshot remains stable to callers.
  - Whether the temporary root seam is represented by an explicit placeholder branch or by a tiny owner-private helper; what must not happen is a separate fake root-owner shell.

The creator must keep the owner boundary narrow:

- `ExfatFs` is the canonical filesystem-wide owner for the refactor.
- `sb()` is a projection of already-owned filesystem state, not a place to reparse boot geometry.
- `root_inode()` is a named temporary seam on the owner itself, not a reason to introduce a separate root shell.
- `sync()` is a placeholder, not the first stage of real writeback policy.

## Functional Specification

### Operation

- Name: `ExfatFs::name`
- Inputs:
  - `&self`
- Actions:
  - Return the canonical exFAT filesystem name used by the VFS surface.
  - Keep the value stable for the lifetime of the owner.
  - Do not vary the name by mount policy, device identity, or runtime statistics.
- Outputs:
  - `&'static str`
- Postconditions:
  - Callers can treat the filesystem name as a fixed identity string for the refactor owner.

### Operation

- Name: `ExfatFs::sb`
- Inputs:
  - `&self`
- Actions:
  - Return a `SuperBlock` snapshot derived from the owner’s already-normalized filesystem state.
  - Preserve the stable runtime geometry and container identity facts that later VFS callers need.
  - Do not rebuild the filesystem from disk and do not mutate owner state while producing the snapshot.
- Outputs:
  - `SuperBlock`
- Postconditions:
  - Repeated calls return equivalent snapshots while the owner state is unchanged.

### Operation

- Name: `ExfatFs::fs_event_subscriber_stats`
- Inputs:
  - `&self`
- Actions:
  - Return the owner’s single `FsEventSubscriberStats` object.
  - Keep the returned reference stable across calls.
  - Do not synthesize a fresh stats wrapper per call.
- Outputs:
  - `&FsEventSubscriberStats`
- Postconditions:
  - Subscriber accounting remains attached to the same filesystem owner object.

### Operation

- Name: `ExfatFs::root_inode`
- Inputs:
  - `&self`
- Actions:
  - Keep the method as the explicit temporary seam that later root-handoff work will replace.
  - Use the architect-approved comment exactly:

    ```rust
    // Temporary seam: EXR-FS-OPEN-22 will install the real root inode after EXR-INODE-CORE-17 lands.
    ```

  - Do not move root construction into a helper shell or hide it behind a fake owner type.
  - Do not expand this seam into mount/open sequencing, inode cache ownership, or directory discovery.
- Outputs:
  - `Arc<dyn Inode>`
- Exit plan:
  - `EXR-FS-OPEN-22` absorbs the real root construction after `EXR-INODE-CORE-17` provides the inode carrier.

### Operation

- Name: `ExfatFs::sync`
- Inputs:
  - `&self`
- Actions:
  - Keep `sync()` as an explicit placeholder.
  - It may return success without performing real flush ordering.
  - It must not pull in inode-cache traversal, bitmap flushing, allocation policy, or dirty-state ordering.
- Outputs:
  - `Result<()>`
- Postconditions:
  - The method remains a narrow filesystem-owner seam until `EXR-SYNC-31` owns the real ordering work.

## Invariants

- `ExfatFs` is the single canonical filesystem owner for this refactor component.
- The `FileSystem` identity surface is stable and does not depend on re-reading the disk.
- `sb()` returns a snapshot of already-owned normalized state, not a fresh mount operation.
- `fs_event_subscriber_stats()` always references the same owner-owned stats object.
- `root_inode()` is the only temporary seam on this filesystem-owner boundary.
- `sync()` does not define or imply real flush ordering.
- No helper or wrapper should be introduced solely to expose stored fields that the `FileSystem` trait already covers.

## Concurrency Specification

- Shared state:
  - The owner’s filesystem-wide runtime state, including the normalized superblock snapshot and subscriber stats.
- Lock ordering:
  - None is established by this component.
- Atomicity requirements:
  - The public owner-surface methods must appear stable to callers, but this component does not define a new lock hierarchy or a separate async protocol.
- Forbidden interleavings:
  - Do not smuggle inode-cache, allocation, or writeback ordering into `sync()`.
  - Do not let `root_inode()` become a hidden mount shell.
- Allowed simplifications:
  - A trivial placeholder `sync()` is acceptable for now.
  - No dedicated `02_designer_async.md` is needed because this component does not introduce a new concurrency contract beyond the placeholder `sync()` seam, and the remaining serialization assumptions are recorded here.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add the `ExfatFs` owner type in `fs.rs`.
  - Store the stable owner state needed for the `FileSystem` surface and the temporary root seam.
  - Implement `name()`, `sb()`, and `fs_event_subscriber_stats()`.
  - Implement `root_inode()` with the exact temporary comment and a temporary seam only.
  - Implement `sync()` as a trivial placeholder that does not introduce real flush ordering.
  - Preserve inherited `FileSystem` defaults for `source()`, `flags()`, and `set_fs_flags()` unless a later component truly owns them.
  - Wire any module declaration in `mod.rs` only if the new file requires it, and serialize that declaration edit with `EXR-INODE-CORE-17`.
- Explicit non-goals:
  - No mount/open sequencing.
  - No inode cache, directory service, allocation, upcase, bitmap, or page-cache behavior.
  - No dedicated root-owner shell or stats shell.
  - No real sync manager or writeback policy.

### Serial Checker Pass

- Required checker-owned tests:
  - A stability test that exercises `name()` and `sb()` on the same owner instance and confirms the snapshot is stable across repeated calls.
  - A stats test that confirms `fs_event_subscriber_stats()` returns the same owner-owned object across calls and remains unchanged by `sync()`.
  - A seam regression that keeps `root_inode()` exposed on `ExfatFs` itself rather than behind a helper shell or alternate owner object.
  - A placeholder-sync regression that confirms `sync()` does not require flush-order state or mutate the stable owner snapshot.
- Observable properties that must pass before leaving the serial loop:
  - The owner still behaves like one filesystem object, not a collection of unrelated helper wrappers.
  - The temporary root seam remains explicit and local to the owner boundary.
  - `sync()` is still clearly a placeholder and not an early flush-order implementation.

### Concurrency Creator Pass

- Required implementation obligations:
  - No dedicated concurrency implementation required.
- Explicit non-goals:
  - Do not add locks, atomics, background maintenance, or async-facing orchestration for this component.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - The component remains a single filesystem-owner boundary with the temporary seam and placeholder `sync()` recorded explicitly in this artifact.

## Acceptance Notes

- The reviewer should confirm that `root_inode()` is still marked as a temporary seam with the exact `EXR-FS-OPEN-22` comment.
- The reviewer should reject any attempt to turn `sync()` into a hidden flush-order owner before `EXR-SYNC-31`.
- The reviewer should treat a separate root shell or stats shell as a boundary regression.
- Any `mod.rs` declaration edit needed to wire the new owner file must stay serialized with the sibling inode lane rather than being treated as file-parallel work.
