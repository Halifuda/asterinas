<!-- SPDX-License-Identifier: MPL-2.0 -->

# Meso-Component Designer KTest: `mount_volume_state`

*This artifact defines the exact testing obligations for the `Checker`. It must separate Creator-synced unit obligations from independent meso-level integration obligations. Each scenario should explain `Setup`, `Execution Chain`, and `Assertion` at a high level only, without line-by-line implementation detail.*

## 1. Creator-Synced Unit Test Obligations

### Unit Scenario: Base-Case Success
- **Related Micro-Features:** `Boot region validation and parameter load at mount`; `Allocation bitmap is the free-space truth source`; `Up-case Table is the durable case-folding truth source`; `Mount option defaults and remount mutability boundary`; `Superblock counters and statfs reflect cached cluster accounting`; `Asterinas mount lifecycle must eagerly expose root inode and global sync state`.
- **Setup:** Provide a minimal valid exFAT image or mocked block-device fixture with valid main boot-region checksum/geometry, one accepted Allocation Bitmap, one valid Up-case Table, a valid root-directory anchor, and mount options at their defaults.
- **Execution Chain:** Call the single exported Meso-Level Interface.
- **Assertion:** Mount returns success, publishes exactly one root inode carrier, exposes superblock geometry/free-space counters derived from accepted mount state, stores mount options at expected defaults, and leaves lookup-steady-state behavior to later meso components.

### Unit Scenario: Error Paths
- **Related Micro-Features:** `Boot region validation and parameter load at mount`; `Allocation bitmap is the free-space truth source`; `Up-case Table is the durable case-folding truth source`; `Mount option defaults and remount mutability boundary`; `Mount-time accounting may fall back to recount under corruption-recovery conditions`.
- **Scenario [Invalid Boot Region]:**
  - **Setup:** Corrupt the main boot checksum, cluster geometry, FAT geometry, or root-directory anchor in the mount fixture.
  - **Execution Chain:** Call the single exported Meso-Level Interface for mount bootstrap.
  - **Assertion:** Returns `Err(MountVolumeStateError::InvalidOnDiskLayout)` for structural invalidity or `Err(MountVolumeStateError::DeviceIo)` for injected read failure, publishes no root inode, and exposes no partial `sb()` state.
- **Scenario [Invalid Allocation Bitmap / Recount Failure]:**
  - **Setup:** Provide a fixture where the Allocation Bitmap seed is inconsistent or the recount path hits an injected block-device failure.
  - **Execution Chain:** Call the single exported Meso-Level Interface for mount bootstrap.
  - **Assertion:** A successful recount produces coherent counters before publication; unrecoverable allocator inconsistency returns `Err(MountVolumeStateError::InconsistentAccounting)`; injected media failure returns `Err(MountVolumeStateError::DeviceIo)`; neither failure leaks stale counters.
- **Scenario [Invalid Up-case Table]:**
  - **Setup:** Corrupt the Up-case Table checksum or length while keeping boot geometry otherwise valid.
  - **Execution Chain:** Call the single exported Meso-Level Interface for mount bootstrap.
  - **Assertion:** Mount fails with `Err(MountVolumeStateError::InvalidOnDiskLayout)` before publishing the lookup naming truth or root inode.
- **Scenario [Unsupported Remount Delta]:**
  - **Setup:** Mount a valid instance, then request mutation of a dentry/inode-semantic option that is fixed after mount.
  - **Execution Chain:** Call the single exported Meso-Level Interface for remount policy validation.
  - **Assertion:** Returns `Err(MountVolumeStateError::UnsupportedRemountDelta)` and leaves the original policy and `sb()` state unchanged.

## 2. Invariant / Rollback Obligations

*Tests required to certify memory safety, structural coherence (e.g., FAT chain linkage), and rollback stability. These obligations may be implemented in Creator-synced passes when they map cleanly to the covered micro set.*
### Invariant Scenario 1
- **Related Micro-Features:** `Asterinas mount lifecycle must eagerly expose root inode and global sync state`; `Superblock counters and statfs reflect cached cluster accounting`.
- **Setup:** Mount a valid fixture and obtain the mounted instance through the meso interface.
- **Execution Chain:** Invoke the meso interface for root retrieval and superblock snapshot multiple times.
- **Assertion:** Root retrieval always returns the same root identity for the mounted instance; `sb()` snapshots are internally coherent and do not trigger ordinary lookup or a fresh full bitmap scan per call.

### Invariant Scenario 2
- **Related Micro-Features:** `VolumeDirty marks in-flight versus quiesced global state`; `VolumeFlags also carries media-failure and clear-before-modify state`.
- **Setup:** Mount fixtures with combinations of `VolumeDirty`, `MediaFailure`, and `ClearToZero` set in the live Main Boot `VolumeFlags`.
- **Execution Chain:** Call the single exported Meso-Level Interface for mount bootstrap, then query the mount-visible state through the same meso boundary.
- **Assertion:** The anomaly posture is preserved as mount-visible state for later meso components; the test must not expect this meso to clear dirty bits, repair clear-to-zero state, or perform forced-shutdown transitions.

### Invariant Scenario 3
- **Related Micro-Features:** `Mount option defaults and remount mutability boundary`.
- **Setup:** Mount a valid fixture, snapshot mount policy, then perform one accepted `discard` remount change followed by one rejected immutable-option change.
- **Execution Chain:** Call the single exported Meso-Level Interface for each remount request.
- **Assertion:** The accepted mutable option linearizes visibly; the rejected immutable change leaves all previous option fields unchanged.

## 3. Meso-Level Integration Test Obligations

*Each integration scenario must involve tightly coupled micro-features and is implemented as an independent Checker pass. The `Success Path` entry is mandatory whenever the meso-component has more than trivial cross-micro interaction. The other three path types are optional depending on complexity; if omitted, explain why.*

### Success Path (Mandatory)
- **Covered Micro-Features:** `Boot region validation and parameter load at mount`; `Allocation bitmap is the free-space truth source`; `Up-case Table is the durable case-folding truth source`; `Superblock counters and statfs reflect cached cluster accounting`; `Asterinas mount lifecycle must eagerly expose root inode and global sync state`.
- **Setup:** Use a valid image fixture with non-zero allocated clusters, valid naming truth, and a root directory that can be published.
- **Execution Chain:** Mount through `mount_volume_state`, request the root inode, request `sb()`, and hand the published naming truth to a lookup-facing smoke path only as a consumer check.
- **Assertion:** Mount succeeds, root is synchronously available, `sb()` reports geometry/free-space derived from accepted mount state, and no steady-state lookup or allocator ownership is pulled into the mount meso.

### Failure-Maintenance Path (Optional)
- **Required?:** Yes; mount bootstrap has many partial-validation failure points, so rollback/no-publication behavior must be proven.
- **Covered Micro-Features:** `Boot region validation and parameter load at mount`; `Allocation bitmap is the free-space truth source`; `Up-case Table is the durable case-folding truth source`; `Mount-time accounting may fall back to recount under corruption-recovery conditions`.
- **Setup:** Prepare fixtures that fail at successive phases: boot validation, bitmap seed/recount, Up-case Table validation, and root-directory anchor publication.
- **Execution Chain:** Attempt mount through the meso interface for each fixture.
- **Assertion:** Each failure returns the exact contract variant for its class (`MountVolumeStateError::InvalidOnDiskLayout`, `MountVolumeStateError::DeviceIo`, or `MountVolumeStateError::InconsistentAccounting`), publishes no root inode, leaves no callable `sb()` snapshot for the failed mount, and does not retain partially accepted global state.

### Idempotence / Repeated-Call Path (Optional)
- **Required?:** Yes; `root_inode()` and `sb()` are stable VFS-facing projections after mount.
- **Covered Micro-Features:** `Asterinas mount lifecycle must eagerly expose root inode and global sync state`; `Superblock counters and statfs reflect cached cluster accounting`.
- **Setup:** Mount a valid fixture and keep the mounted instance alive.
- **Execution Chain:** Repeatedly call the meso interface for root retrieval and superblock snapshot without intervening mutation.
- **Assertion:** Root identity remains stable, `sb()` remains coherent, and repeated reads do not allocate duplicate root objects or re-run mount-time media validation.

### Concurrency Path (Optional)
- **Required?:** Yes; remount and superblock/root readers share `ExfatFs state rwlock` and must linearize without lower-lock inversions.
- **Covered Micro-Features:** `Mount option defaults and remount mutability boundary`; `Asterinas mount lifecycle must eagerly expose root inode and global sync state`; `Superblock counters and statfs reflect cached cluster accounting`.
- **Setup:** Mount a valid fixture, then arrange concurrent root retrieval, superblock snapshot, and accepted/rejected remount operations.
- **Execution Chain:** Call the meso interface from concurrent ktest tasks using the same mounted instance.
- **Assertion:** No deadlock occurs, readers observe either the old or new remount-admitted policy consistently, rejected remount deltas do not leak partial policy state, and no lower-level inode or allocator lock is required before entering the meso interface.
