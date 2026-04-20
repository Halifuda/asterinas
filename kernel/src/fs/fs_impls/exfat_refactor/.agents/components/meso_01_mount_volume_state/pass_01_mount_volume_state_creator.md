<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Pass Implementation Report: `mount_volume_state`

*This artifact acts as the Creator's receipt, documenting how the Designer's strict specifications were actually implemented in Rust for one explicit Creator Pass. It serves as context for the synchronized Checker's validation and the Reviewer's static checks.*

## 1. Pass Identity & Write-Set

**Creator Pass ID:** `pass_01_mount_volume_state`
**Parent Meso-Component:** `meso_01_mount_volume_state`
**Covered Micro-Features:**
- `Boot region validation and parameter load at mount`
- `Allocation bitmap is the free-space truth source`
- `VolumeDirty marks in-flight versus quiesced global state`
- `VolumeFlags also carries media-failure and clear-before-modify state`
- `Up-case Table is the durable case-folding truth source`
- `Mount option defaults and remount mutability boundary`
- `Superblock counters and statfs reflect cached cluster accounting`
- `Asterinas mount lifecycle must eagerly expose root inode and global sync state`
- `Mount-time accounting may fall back to recount under corruption-recovery conditions`
**Source Files Modified/Created:**
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs` (created the refactor-owned module shell and local init exposure)
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` (implemented `ExfatFs`, `mount_volume_state(...)`, mount/remount state publication, VFS `FileSystem` projection, and the checker-only `BioSegment` slice fix for the in-tree memory disk)
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` (implemented the eagerly published root `ExfatInode` carrier required by the mount lifecycle)
- `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs` (implemented boot-region validation, root-directory bootstrap scanning, bitmap accounting seed/recount admission, Up-case Table loading, and repaired bitmap walking to stop at the recorded stream length)
- `kernel/src/fs/fs_impls/mod.rs` (added module exposure only; no registration change)

## 2. Pass Coverage & Contract Satisfaction

- `mount_volume_state(...)` is now the single refactor-owned meso entry point. It accepts `Candidate` mount bootstrap input, validates on-disk state before publication, publishes one coherent `ExfatFs`, and serves the published-state `RootInode`, `SuperBlock`, `Flags`, and `Remount` branches from the same boundary.
- Mount bootstrap validates the Main Boot region fields, checks the boot checksum, enforces geometry/root-cluster bounds, reads live `VolumeFlags`, scans the root directory for the Allocation Bitmap and Up-case Table entries, loads the Up-case Table bytes, and seeds cached cluster accounting from the Allocation Bitmap.
- Allocation Bitmap accounting is cached at mount and later projected through `sb()` without rescanning the bitmap on every query. When the persisted `PercentInUse` hint is missing or does not match the counted bitmap usage, the implementation records that the cached counter came from recount-style admission rather than trusting the hint.
- Checker repair 03 tightened the Allocation Bitmap chain walk so accounting consumes exactly the validated bitmap stream length, validates padding bits inside the stream, and does not reject unrelated bytes in the final physical cluster after the stream ends.
- Checker repair 05 leaves the production checksum math unchanged and instead repairs the `#[cfg(ktest)]` in-tree `BlockDevice` shim used by the pass tests: unaligned-in-page reads and writes now honor each `BioSegment` slice window rather than the whole backing DMA object, so the checksum-sector read at sector 11 observes the fixture bytes that the Checker reported.
- The published `ExfatFs` exposes one eagerly created root inode carrier immediately at mount completion. The root carrier is intentionally limited to mount-visible behavior for this pass: it provides root identity, `.` / `..` enumeration, and root lookup for those synthetic names without pulling in downstream directory lookup/mutation ownership.
- Remount handling is linearized under the filesystem state lock. The current pass admits the `discard` option bit as mutable and rejects unsupported `FsFlags` deltas; it also preserves the read-only conflict boundary by rejecting read-only to read-write transitions once the published state is read-only.
- `FileSystem::sync()` only forwards the block-device flush. Dirty-state clearing and steady-state volume-state choreography remain outside this pass, consistent with the Designer boundary and the later `meso_08` ownership split.

## 3. Lock Orchestration & RAII Notes

- Mount bootstrap performs all block I/O before any published `ExfatFs` state becomes reachable. The boot-region reads, FAT walks, Allocation Bitmap counting, and Up-case Table reads happen without taking the published filesystem state lock, so no published lock is held across blocking media I/O.
- The repaired bitmap walk keeps the existing pre-publication block-I/O discipline and uses closure-local counters only; it introduces no new locks and still returns before any `ExfatFs` state lock exists.
- The repair-05 test-disk change is confined to checker-only fixture I/O and introduces no production lock or publication changes. It only makes the synthetic `BlockDevice` honor `BioSegment` slice offsets for checksum-sector reads.
- Publication remains all-or-nothing. The code builds validated mount inputs first, creates the root inode carrier, and then publishes allocator and state snapshots only after the bootstrap inputs are mutually coherent.
- Published-state projections follow the Designer order locally: `sb()` takes the filesystem state lock first, then the allocator lock, and builds the superblock snapshot from those cached values. `root_inode()` and `flags()` only read the published filesystem state.
- Remount linearization occurs under the filesystem state write lock alone. The pass does not enter allocator or inode locks during remount and does not perform any blocking I/O in that path.

## 4. Helper & Local Type Inventory

| Introduced Symbol | Type (Helper/Struct/Enum) | Whitelist Rule (A/B/C) & Justification |
|-------------------|---------------------------|-----------------------------------------|
| `ExfatFs` | Local struct | **Rule A**: Isolates published filesystem state and allocator state behind the Designer-required lock boundary so bootstrap I/O can complete before publication. |
| `PublishedMountState` | Local struct | **Rule A**: Separates mount-visible publication data from pre-publication bootstrap material, keeping the published state lock scope narrow and explicit. |
| `AllocatorState` | Local struct | **Rule A**: Keeps cached cluster accounting behind its own lockable state so `sb()` can project coherent counters without rescanning the bitmap. |
| `ExfatInode` | Local struct | **Rule C**: Provides the concrete root `Inode` trait carrier required by the VFS mount lifecycle. |
| `BootRegion` | Local struct | **Rule B**: The validated geometry is reused by boot validation, cluster offset calculation, stream validation, and superblock projection within this meso. |
| `AllocationBitmapRecord` | Local struct | **Rule B**: The parsed bitmap record is reused by mount-time validation, accounting seed, and cached allocator publication. |
| `UpcaseTable` | Local struct | **Rule B**: The loaded naming-truth data is reused after mount publication instead of being transient bootstrap-only data. |
| `FatReader` | Private helper struct | **Rule B**: FAT entry lookups are reused by root-directory scanning, Allocation Bitmap traversal, and Up-case Table loading within this meso. |
| `walk_cluster_chain` | Private helper fn | **Rule C**: Uses a localized callback shape to share one validated cluster-chain traversal path across root-directory scan, bitmap counting, and Up-case Table loading. |
| `ExfatMountOptions` | Local struct | **Rule A**: Carries mount-policy state through bootstrap/remount without widening the public meso interface beyond the Designer contract. |
| `ValidatedMount` | Local struct | **Rule A**: Bundles validated bootstrap outputs so publication can happen in one final step after all blocking I/O has finished. |

## 5. Contract Deviations & Boundary Notes

- **Incidental Supporting Edits Outside Covered Micro-Features:** `kernel/src/fs/fs_impls/mod.rs` now exposes the `exfat_refactor` module so the new Rust files are compiled with the rest of `fs_impls`, and the `#[cfg(ktest)]` memory-disk path in `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` now copies through each `BioSegment` slice instead of the whole DMA object so the checker fixture read path matches the mounted image bytes. I intentionally did **not** call `exfat_refactor::init()` from `fs_impls::init()` so the legacy registered `exfat` filesystem remains active, matching protocol rule 7.
- **Deviations:** The Designer contract fixes the `Candidate` target shape but does not provide a separate initial `FsFlags` carrier. To preserve the exact meso entry signature without widening it, the refactor-owned `ExfatMountOptions` stores the initial mount `FsFlags` during bootstrap, while remount still uses the explicit `next_flags` argument from the contract.
- **Unresolved Ambiguities:** The Designer contract names the mutable remount boundary explicitly only for `discard`. This pass therefore treats `discard` as the only mutable option bit and rejects other `FsFlags` deltas outside the read-only transition check rather than inventing additional remount policy.
