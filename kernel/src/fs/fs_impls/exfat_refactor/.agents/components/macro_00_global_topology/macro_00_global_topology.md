<!-- SPDX-License-Identifier: MPL-2.0 -->

# Macro 00: Global Topology

## 1. Identified On-disk Structure Owners

- `BootRegionAndVolumeFlags`: Main/Backup Boot regions, validated geometry fields, root-directory anchor, and the persisted `VolumeFlags` state bits (`VolumeDirty`, `MediaFailure`, `ClearToZero`, `ActiveFat`, stale-backup rule).
- `AllocationBitmap`: The durable free-space truth source plus its directory-entry metadata and bitmap payload.
- `FileAllocationTable`: The durable FAT chain graph for non-contiguous stream mapping and directory/file cluster traversal.
- `UpcaseTable`: The persisted case-folding truth source and checksum-validated table payload used for exFAT name matching.
- `DirectoryEntrySet`: The contiguous primary-plus-secondary directory-entry set, including `SecondaryCount`, `SetChecksum`, file-name entries, deletion markers, and unrecognized-entry typing.
- `StreamExtension`: The persisted stream-shape record carrying `AllocationPossible`, `NoFatChain`, `ValidDataLength`, `FirstCluster`, `DataLength`, and `NameHash`.
- `FileEntryMetadata`: The durable DOS attribute and timestamp fields stored in file/directory entry sets.
- `VolumeIdentityEntries`: The dedicated Volume Label and Volume GUID directory entries used for administrative identity.

## 2. Identified Runtime Owners

- `ExfatFs`: Per-mount runtime authority for validated geometry, mount/remount policy, superblock counters, root-object creation, filesystem-wide sync, anomaly-state interpretation, and volume-scoped administrative surfaces.
- `ExfatInode(file)`: Runtime authority for ordinary file identity, content extent state, block mapping, cached/direct I/O boundaries, metadata projection for regular files, and file-scoped sync surfaces.
- `ExfatInode(dir)`: Runtime authority for directory identity, lookup/readdir traversal, namespace mutation, directory-stream growth, and metadata projection for directories.

## 3. On-disk Structure Owner -> Runtime Owner Projection

| On-disk Structure Owner | Primary Runtime Owner | Secondary Runtime Owner(s) / Notes | Why this projection exists |
|---|---|---|---|
| `BootRegionAndVolumeFlags` | `ExfatFs` | `ExfatInode(file)` and `ExfatInode(dir)` only consume derived mount or anomaly state | Boot geometry, mount policy, global dirty/media-failure interpretation, and filesystem-wide sync all belong to the mount-scoped authority rather than a single inode. |
| `AllocationBitmap` | `ExfatFs` | `ExfatInode(file)`, `ExfatInode(dir)` request allocation/free through `ExfatFs`-owned coordination | Free-space truth, `used_clusters`, `statfs`, online discard, recount fallback, and allocation conflict prevention are mount-global concerns. |
| `FileAllocationTable` | `ExfatFs` | `ExfatInode(file)` and `ExfatInode(dir)` depend on it for stream and directory traversal | The FAT is a shared global chain graph, so its serialization and corruption boundaries cannot be owned independently by one inode instance. |
| `UpcaseTable` | `ExfatFs` | `ExfatInode(dir)` consumes the loaded table during lookup and namespace mutation | The table is mounted once as a durable naming truth and then reused by many directories. |
| `DirectoryEntrySet` | `ExfatInode(dir)` | `ExfatFs` for root-directory bootstrap and global anomaly policy; `ExfatInode(file)` for self-metadata rewrites reached through the parent directory view | Namespace visibility, entry-set slot management, rename sequencing, emptiness checks, and unrecognized-entry boundaries are organized around directory streams. |
| `StreamExtension` | `ExfatInode(file)` | `ExfatInode(dir)` for directory-stream shape; `ExfatFs` for allocator coordination notes | The stream record is the closest durable truth for logical size, initialized-data extent, `NoFatChain`, and physical start cluster. |
| `FileEntryMetadata` | `ExfatInode(file)` | `ExfatInode(dir)` for directory entry-set metadata; `ExfatFs` for mount-derived uid/gid/mode/timezone policy inputs | Regular-file metadata projection is the primary rewrite path for this durable metadata family, while directory metadata follows the same durable format under a sibling inode owner and the visible POSIX projection still depends partly on mount policy. |
| `VolumeIdentityEntries` | `ExfatFs` | Temporary administrative carrier seams, if later required by the target VFS, stay subordinate to `ExfatFs` rather than becoming independent macro owners | Volume label/GUID are volume-scoped administrative identity, not ordinary namespace entries of one file inode. |

## 4. Candidate Meso-Component Index

| Candidate Meso-Component | Primary Runtime Owner | Entry-Surface Family | Durable Touch-Set | Static Lock Envelope | Why this is one meso boundary |
|---|---|---|---|---|---|
| `mount_volume_state` | `ExfatFs` | `mount`, `remount`, root bootstrap, superblock query | `BootRegionAndVolumeFlags`, `AllocationBitmap`, `UpcaseTable`, root `DirectoryEntrySet` anchor | `ExfatFs state rwlock` only; may descend later into lower allocator or root-directory read state but may not start from inode locks | These features share one mount/bootstrap contract: validate geometry, seed global runtime state, expose root, and interpret anomaly flags before ordinary inode work begins. |
| `free_space_accounting_and_discard` | `ExfatFs` | allocator accounting, `statfs`, online discard, `FITRIM`, recount fallback | `AllocationBitmap`, `FileAllocationTable`, `BootRegionAndVolumeFlags` notes for anomaly overlay | `ExfatFs state rwlock` -> `ExfatFs allocator rwlock`; no inode lock may be acquired after entering the allocator critical state | Cached `used_clusters`, free/alloc serialization, discard policy, and corruption-triggered recount all share one global free-space contract. |
| `directory_lookup_and_identity` | `ExfatInode(dir)` | `lookup`, `readdir`, alias reconciliation, negative-cache revalidation | `DirectoryEntrySet`, `UpcaseTable`, `StreamExtension` (`NameHash`, name length) | `ExfatFs state rwlock (read)` -> directory `InodeRwLock(Read)`; no allocator entry | Name folding, stable identity reconstruction, alias reuse, trailing-dot policy, and unrecognized-entry typing are one read-side namespace contract. |
| `directory_entry_mutation` | `ExfatInode(dir)` | `create`, `mkdir`, `unlink`, `rmdir`, `rename`, explicit refusal of unsupported tree operations | `DirectoryEntrySet`, directory `StreamExtension`, `AllocationBitmap`, `FileAllocationTable`, `FileEntryMetadata` | `ExfatFs state rwlock (write/intend-mutate)` -> ordered directory `InodeRwLock(Write)` set -> optional per-stream extent lock -> `ExfatFs allocator rwlock` | Slot acquisition, namespace invalidation order, cross-directory move sequencing, emptiness gates, newborn-directory shape, and cluster reclamation belong to one tree-mutation failure domain. |
| `file_content_mapping_and_cached_io` | `ExfatInode(file)` | `read_at`, cached write/read preparation, `bmap`, page-cache backend mapping | `StreamExtension`, `FileAllocationTable` | `ExfatFs state rwlock (read)` -> file `InodeRwLock(Read)` -> file stream-extent rwlock (`Read` for map / cached-read paths) | Logical block mapping, cached-I/O reuse, valid-size guards, and the truncate anti-race boundary share one file-read/mapping contract. |
| `file_content_mutation` | `ExfatInode(file)` | `write_at`, `resize`, append growth, truncate/shrink, fallocate-family refusal | `StreamExtension`, `AllocationBitmap`, `FileAllocationTable`, `DirectoryEntrySet`, `FileEntryMetadata`, `BootRegionAndVolumeFlags` overlay | `ExfatFs state rwlock (write/intend-mutate)` -> file `InodeRwLock(Write)` -> file stream-extent rwlock (`Write`) -> `ExfatFs allocator rwlock` | Zero-fill before exposure, `NoFatChain` flips, append ordering, shrink semantics, timestamp updates, and explicit fallocate refusal must live under one mutation contract. |
| `file_sync_and_persistence` | `ExfatInode(file)` | file-scoped `fsync`, `sync_data`, and ordinary-file flush/fail-fast surfaces | `DirectoryEntrySet`, `StreamExtension`, dirty page-cache state, backing block-device flush state | `ExfatFs state rwlock (read or write depending on dirty-state intent)` -> file `InodeRwLock(Read/Write as needed)`; may enter block-I/O waits but never under `SpinLock` | This meso exists for one regular file's persistence boundary: it flushes that file's dirty data/metadata view and honors file-local fast-fail conditions without owning volume-wide quiesce decisions. |
| `filesystem_sync_and_volume_state` | `ExfatFs` | filesystem-wide `sync`, clean unmount/quiesce, mount-time anomaly interpretation, and volume-state transitions | `BootRegionAndVolumeFlags`, `AllocationBitmap`, global dirty-state bookkeeping | `ExfatFs state rwlock` -> optional lower inode / allocator locks in hierarchy order | This remains distinct because its owner and failure domain are mount-scoped: it brackets whole-filesystem quiesce, VolumeDirty transitions, and anomaly-state interpretation across many inodes rather than one file's flush path. |
| `file_metadata_projection_and_update` | `ExfatInode(file)` | regular-file metadata getters/setters, `chmod`, ownership projection, timestamp encode/decode | `FileEntryMetadata`, `DirectoryEntrySet`, mount-derived policy from `ExfatFs` | `ExfatFs state rwlock (read for projection / write for mutable policy-sensitive updates)` -> file `InodeRwLock(Read/Write)` | Mount-derived uid/gid/mode policy, DOS `ReadOnly` propagation, timezone translation, and `allow_utime` policy form one metadata contract for regular files distinct from content mutation. |
| `directory_metadata_projection_and_update` | `ExfatInode(dir)` | directory metadata getters/setters, ownership projection, timestamp encode/decode | `FileEntryMetadata`, `DirectoryEntrySet`, mount-derived policy from `ExfatFs` | `ExfatFs state rwlock (read for projection / write for mutable policy-sensitive updates)` -> directory `InodeRwLock(Read/Write)` | Directory metadata uses the same durable entry-set fields but belongs to the directory runtime owner, especially where namespace mutation and directory-specific timestamps meet mount-derived policy. |
| `volume_admin_identity` | `ExfatFs` | volume-label / GUID get-set, forced-shutdown control, administrative carrier/refusal routing | `VolumeIdentityEntries`, `BootRegionAndVolumeFlags`, `AllocationBitmap` notes for trim/discard control | `ExfatFs state rwlock (write for mutators)` -> optional root-directory `InodeRwLock(Write)` when rewriting directory-backed identity entries | Volume label/GUID, forced-shutdown control entry, and other volume-scoped administrative surfaces share one privileged administrative contract that should not be conflated with ordinary namespace mutation. |

## 5. Global Lock Topology & Hierarchy

- **Level 1 (Top Level)**: `ExfatFs state rwlock`
  - Protects mount-established runtime state, read-only / shutdown / remount-visible policy, root bootstrap validity, and global anomaly-state interpretation.
- **Level 2**: `ExfatInode rwlock`
  - Protects one inode's namespace-facing or metadata-facing state. When multiple inode locks are required, they must be acquired in one stable order derived from durable identity (directory before non-directory when both are needed; peers ordered by stable inode identity, never ad hoc by call path).
- **Level 3**: `ExfatStream extent rwlock`
  - Per-stream anti-race boundary for logical block mapping versus resize/truncate/mutation of cluster relationships.
- **Level 4 (Lowest)**: `ExfatFs allocator rwlock`
  - Protects mount-global free-space accounting, Allocation Bitmap mutation, FAT-allocation coordination, and discard / recount bookkeeping.

> **Hierarchy Rule:** A thread holding a Level N lock MAY NOT acquire a Level M lock if M <= N.

Additional static lock notes:

- `SpinLock` is not part of the exFAT mutable-data hierarchy. No meso component may require holding a `SpinLock` across block I/O submission, `BioWaiter::wait()`, or any other yielding boundary.
- A meso component may omit higher levels when it only consumes lower-level state already admitted by the hierarchy, but it may never invert the declared order.
- Phase 2 artifacts may refine inlet expectations inside this hierarchy, but they may not add a new higher-level mutable lock above `ExfatFs state rwlock` or a lower mutable lock beneath `ExfatFs allocator rwlock`.

## 6. Structural Invariants

- Every accepted Phase 2 meso artifact must trace back to one row in Section 4 and must not invent a new primary Runtime Owner or a new On-disk Structure Owner.
- Every accepted Phase 2 meso artifact must show how its micro-features map to the On-disk Structure Owners in Section 3; owner gaps are forbidden.
- `VolumeDirty`, `MediaFailure`, `ClearToZero`, forced shutdown, recount fallback, and unrecognized-directory-entry typing are cross-cutting overlay obligations. They attach to the meso components that trigger them; they are not independent macro owners.
- If a temporary administrative adapter is needed to carry Linux-shaped management ABI into Asterinas, it remains a subordinate carrier under `ExfatFs` authority and does not become a separate primary Runtime Owner in Phase 1 topology.
- `ClearToZero` remains a pre-modification obligation above individual write helpers: any Phase 2 mutator touching filesystem structures, directories, or files must account for it in its static boundary notes without redefining the owner map.
- Directory-entry identity remains location-derived, not pathname-derived. Any meso component that can move an entry set must preserve the no-duplicate-identity invariant across lookup, rename, and cache reuse.
- `AllocationBitmap` remains the free-space truth source even when free-space reporting falls back to recount; `statfs` and allocator contracts may not switch to an unrelated durable authority.
- `UpcaseTable` remains the durable case-folding truth source for non-UTF-8-specific paths; Phase 2 lookup or rename artifacts may not replace it with a transient cache as the architectural authority.
- No Architect artifact below this file may prescribe dynamic lock acquisition choreography, helper layout, or Creator-pass slicing; those remain Phase 2 Designer or main-agent concerns.
