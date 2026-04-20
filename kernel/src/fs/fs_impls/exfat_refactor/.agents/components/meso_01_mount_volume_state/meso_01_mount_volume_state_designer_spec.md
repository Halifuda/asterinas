<!-- SPDX-License-Identifier: MPL-2.0 -->

# Meso-Component Designer Specification: `mount_volume_state`

*This artifact dictates the dynamic contract for the Meso-Component. Creator Passes must follow it exactly without inventing external architectures or helpers. When a rule applies only to specific micro-features, name those micro-features explicitly so later pass slicing stays deterministic.*

## 1. Modularity (Rely-Guarantee)

### [GUARANTEE] Meso-Level Interface
*The singular, strict public or crate-visible Rust function signature.*
```rust
pub(crate) fn mount_volume_state(
    target: MountVolumeStateTarget<'_>,
    operation: MountVolumeStateOperation<'_>,
) -> core::result::Result<MountVolumeStateOutcome, MountVolumeStateError>;
```

The contract types are part of this meso boundary and are fixed as follows:

- `MountVolumeStateTarget<'a>`
  - `Candidate { block_device: &'a Arc<dyn BlockDevice>, source: Option<&'a str>, options: &'a ExfatMountOptions }`
  - `Published { fs: &'a ExfatFs }`
- `MountVolumeStateOperation<'a>`
  - `Mount`
  - `Remount { next_flags: FsFlags, next_options: &'a ExfatMountOptions }`
  - `RootInode`
  - `SuperBlock`
  - `Flags`
- `MountVolumeStateOutcome`
  - `Mounted { fs: Arc<ExfatFs>, root_inode: Arc<dyn Inode>, super_block: SuperBlock, flags: FsFlags }`
  - `Remounted { flags: FsFlags }`
  - `RootInode { root_inode: Arc<dyn Inode> }`
  - `SuperBlock { super_block: SuperBlock }`
  - `Flags { flags: FsFlags }`
- `MountVolumeStateError`
  - `InvalidMountInput`
  - `InvalidOnDiskLayout`
  - `DeviceIo`
  - `UnsupportedRemountDelta`
  - `ReadOnlyConflict`
  - `UnpublishedState`
  - `InconsistentAccounting`

No additional crate-visible entry function is permitted for this meso. Boot validation, bitmap seeding, root publication, remount validation, and `SuperBlock` / flags projection must remain internal control flow beneath this single interface.

### [RELY] Bounded Dependencies
*List the explicit OSTD, VFS interfaces, or lower-level capabilities the component is restricted to. Do not use APIs that violate the Architect's lock topology.*
- `Arc<dyn BlockDevice>` synchronous or asynchronous block reads for boot-region, root-directory, Allocation Bitmap, and Up-case Table material needed before publication.
- `BlockDevice::read_blocks`, `BlockDevice::read_blocks_async`, and `BioWaiter::wait()` for media reads; no `SpinLock` may be held across any of these blocking points.
- `ExfatFs state rwlock` as the publication and remount linearization boundary.
- `ExfatFs allocator rwlock` only for one-time `used_clusters` seeding or mount-time recount admission inherited from `meso_01_mount_volume_state_architecture.md`.
- Root-directory `ExfatInode(dir)` construction capability sufficient to publish the root inode once from the validated boot-region anchor.
- VFS `FileSystem` obligations for `root_inode()`, `sb()`, and mount-visible `FsFlags` / remount-state projection from `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md`.
- Neighboring accepted architecture artifacts for boundary handoff: `meso_02_free_space_accounting_and_discard`, `meso_03_directory_lookup_and_identity`, and `meso_08_filesystem_sync_and_volume_state`.

## 2. Functionality (Hoare Logic)

### Pre-conditions
*Logical conditions required of inputs. When applicable, annotate which micro-features depend on each condition.*
- For mount bootstrap, `target` must carry an unmounted block device and uncommitted mount options; no `ExfatFs` instance may have been published from that target yet. Covers `Boot region validation and parameter load at mount`, `Mount option defaults and remount mutability boundary`, and `Asterinas mount lifecycle must eagerly expose root inode and global sync state`.
- For remount-policy validation, `target` must reference a published `ExfatFs` and `operation` must contain only the requested flag/option delta. Covers `Mount option defaults and remount mutability boundary`.
- For `root_inode()` and `sb()` style operations, `target` must reference a published `ExfatFs` whose mount bootstrap already completed successfully. Covers `Asterinas mount lifecycle must eagerly expose root inode and global sync state` and `Superblock counters and statfs reflect cached cluster accounting`.
- Input mount options must be syntactically valid before durable media state is trusted; invalid policy values fail without publishing a partial filesystem. Covers `Mount option defaults and remount mutability boundary`.
- Boot-region geometry, cluster/FAT bounds, root-directory anchor, boot checksum, Allocation Bitmap entry, and Up-case Table entry must pass their validation gates before any public `ExfatFs` or root inode is made reachable. Covers `Boot region validation and parameter load at mount`, `Allocation bitmap is the free-space truth source`, and `Up-case Table is the durable case-folding truth source`.
- Mount-time anomaly bits from `VolumeFlags` must be parsed before mutable state is admitted. Covers `VolumeDirty marks in-flight versus quiesced global state` and `VolumeFlags also carries media-failure and clear-before-modify state`.

### Post-conditions
*Exact success outcomes and defined error variants mapping. When applicable, annotate which micro-features each branch covers.*
- **Case 1 (Mount Success):** Returns `Ok(MountVolumeStateOutcome::Mounted(...))` with one coherent `ExfatFs` instance that owns validated geometry, parsed mount policy, imported anomaly posture, loaded Up-case Table, accepted Allocation Bitmap seed, a cached `used_clusters` seed or recount-admitted value, and a synchronously returnable root inode.
- **Case 2 (Root Retrieval Success):** Returns `Ok(MountVolumeStateOutcome::RootInode(...))` with the same root inode identity for the mounted instance; it must not redo path lookup, consume `meso_03` steady-state lookup, or allocate a second root object.
- **Case 3 (Superblock Snapshot Success):** Returns `Ok(MountVolumeStateOutcome::SuperBlock(...))` derived from accepted mount geometry plus mount-seeded accounting state. It must not perform an ordinary full Allocation Bitmap scan on every call.
- **Case 4 (Remount Success):** Returns `Ok(MountVolumeStateOutcome::Remounted(...))` only when the requested remount delta is within the accepted mutability boundary; dentry/inode-semantic options remain fixed, while `discard` may be admitted as the currently mutable policy bit.
- **Case 5 (Malformed request or wrong target/operation pairing):** Returns `Err(MountVolumeStateError::InvalidMountInput)` when `Mount` is requested on `Published`, when `RootInode` / `SuperBlock` / `Flags` are requested on `Candidate`, or when the remount delta is syntactically malformed before durable-state validation begins. No visible state changes. Covers `INV-VFS-001`, `INV-VFS-004`.
- **Case 6 (Invalid mount media):** Returns `Err(MountVolumeStateError::InvalidOnDiskLayout)` when boot geometry, root anchor, Allocation Bitmap metadata, or Up-case Table structure is invalid. No `ExfatFs` or root inode is published and no partial runtime state becomes visible. Covers `INV-PHY-001`, `INV-PHY-002`, `INV-PHY-005`.
- **Case 7 (Bootstrap or recount I/O failure):** Returns `Err(MountVolumeStateError::DeviceIo)` when block-device reads fail during mount bootstrap or recount admission. No `ExfatFs` or root inode is published and no partial runtime state becomes visible. Covers `INV-PHY-001`, `INV-PHY-002`, `INV-VFS-042`.
- **Case 8 (Unsupported remount delta):** Returns `Err(MountVolumeStateError::UnsupportedRemountDelta)` when the request attempts to mutate semantic mount options other than the admitted `discard` policy. The existing published mount state remains unchanged. Covers `INV-VFS-001`.
- **Case 9 (Read-only policy conflict):** Returns `Err(MountVolumeStateError::ReadOnlyConflict)` when the request would require mutable behavior forbidden by the published read-only posture. The existing published mount state remains unchanged. Covers `INV-VFS-001`.
- **Case 10 (Published-state read on an unmounted instance):** Returns `Err(MountVolumeStateError::UnpublishedState)` when `RootInode`, `SuperBlock`, `Flags`, or `Remount` is requested through `Published` but the referenced `ExfatFs` has not yet completed successful publication. Covers `INV-VFS-004`.
- **Case 11 (Allocator structure inconsistency during seed or recount):** Returns `Err(MountVolumeStateError::InconsistentAccounting)` when allocator structure validation fails in a way that is not a transport I/O fault. No partial counter state may be exposed through `sb()`. Covers `INV-PHY-002`, `INV-VFS-002`, `INV-VFS-042`.

### Invariants
*Integrity rules spanning the execution. When applicable, annotate which micro-features each invariant protects.*
- Publication is all-or-nothing: public `ExfatFs`, `root_inode()`, and `sb()` visibility occurs only after boot validation, global option state, root anchor, naming truth, and accounting seed are mutually coherent. Protects `Boot region validation and parameter load at mount`, `Up-case Table is the durable case-folding truth source`, and `Asterinas mount lifecycle must eagerly expose root inode and global sync state`.
- The Main Boot region is the live mount authority; Backup Boot `VolumeFlags` remain recovery material and must not override the live `VolumeFlags` posture during normal mount success. Protects `Boot region validation and parameter load at mount` and `VolumeDirty marks in-flight versus quiesced global state`.
- Allocation Bitmap remains the durable free-space source for the initial seed; superblock free-space exposure cannot switch to an unrelated durable authority. Protects `Allocation bitmap is the free-space truth source` and `Superblock counters and statfs reflect cached cluster accounting`.
- `used_clusters` is either seeded from validated mount-time bitmap accounting or explicitly admitted through recount fallback before any `sb()` result can claim coherent free-space counters. Protects `Mount-time accounting may fall back to recount under corruption-recovery conditions`.
- Root identity is mount-global and singleton for a mounted instance; `root_inode()` must return the already published root carrier rather than invoking name lookup or constructing duplicates. Protects `Asterinas mount lifecycle must eagerly expose root inode and global sync state`.
- Up-case Table validation is a mount-time prerequisite for later lookup-family contracts; this meso publishes the naming truth but does not perform steady-state lookup comparisons. Protects `Up-case Table is the durable case-folding truth source`.
- Mount-visible anomaly posture is imported and stored without consuming later volume-state ownership: `VolumeDirty`, `MediaFailure`, and `ClearToZero` become inputs to `meso_08` and mutator meso components, not local sync choreography. Protects `VolumeDirty marks in-flight versus quiesced global state` and `VolumeFlags also carries media-failure and clear-before-modify state`.
- Remount must be transactional with respect to mount policy: rejected remount deltas leave all old options, flags, and superblock-visible state unchanged. Protects `Mount option defaults and remount mutability boundary`.

## 3. Dynamic Lock Orchestration

### Inlet/Outlet Lock State
*Inherited from Architect. What static state must the system be in when this executes?*
- **Inlet:** Mount bootstrap enters with no pre-held `ExfatFs state rwlock`, `ExfatInode rwlock`, `ExfatStream extent rwlock`, or `ExfatFs allocator rwlock` because no published state exists yet. Published-state operations enter from the top of the hierarchy with no lower-level locks held.
- **Outlet:** All operations return without holding any `ExfatFs state rwlock`, `ExfatInode rwlock`, `ExfatStream extent rwlock`, or `ExfatFs allocator rwlock`; successful mount leaves a published `ExfatFs` whose state can be re-entered later through Level 1.

### Acquisition Order
*If local locks within the Meso-Component must be acquired, specify the topological order.*
1. `ExfatFs state rwlock` for publication, remount policy changes, root snapshot retrieval, and superblock snapshot retrieval after publication.
2. Root-directory `ExfatInode rwlock` only during root-object publication or read-side root bootstrap that requires the accepted root directory anchor.
3. `ExfatFs allocator rwlock` only during mount-time bitmap seed/recount admission and only after the operation is already rooted in `ExfatFs` state.

The Designer contract forbids acquiring Level 1 after Level 2 or Level 4 has already been entered. If mount bootstrap validates media before the `ExfatFs` state lock exists, no lower-level published lock is considered held; once publication begins, all lock acquisition must follow the frozen macro order.

### Concurrency & Non-blocking Hazards
*State the specific blocking points (e.g., calling `Bio`) and handoffs. Mandate that no deadlocking locks be held across these points.*
- **Hazard 1:** Boot-region, Allocation Bitmap, Up-case Table, and root-directory reads may block on block I/O. No `SpinLock` may be held across those reads or any `BioWaiter::wait()`.
- **Hazard 2:** Fresh mount performs potentially slow media validation before publication. Until the `ExfatFs` instance is published, no caller may observe partial state; after publication starts, state must linearize through the `ExfatFs state rwlock`.
- **Hazard 3:** Mount-time recount can be large. It may hold only the locks allowed by the Architect topology, and any blocking media read must either occur outside lower-level critical sections or be followed by revalidation before the counted result is published.
- **Hazard 4:** `root_inode()` is called eagerly and exactly once by the VFS mount path. The implementation must not rely on later path walking, delayed lookup, or asynchronous root construction to finish root publication.
- **Hazard 5:** `sb()` may be called after mount while allocator state is changing elsewhere. The returned snapshot must be internally coherent under `ExfatFs` state and allocator-accounting constraints, but it must not pull in steady-state allocator mutation logic from `meso_02`.
- **Hazard 6:** Remount policy changes race with readers of mount flags. The accepted or rejected remount delta must be linearized under `ExfatFs state rwlock`, and rejected changes must leave the previous policy visible.
- **Hazard 7:** Anomaly posture is shared with later mutators and sync. This meso may store and expose the mount-time posture, but it must not clear `VolumeDirty`, perform `ClearToZero` repair, or initiate forced-shutdown transitions that belong to `meso_08` / `meso_11`.
