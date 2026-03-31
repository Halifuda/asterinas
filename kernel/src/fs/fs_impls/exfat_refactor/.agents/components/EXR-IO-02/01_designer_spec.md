<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Specification

## Metadata

- Component ID: EXR-IO-02
- Title: Metadata Byte I/O And Cluster Address Translation Helpers
- Status: `Specified`
- Author: designer
- Date: 2026-03-31
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-IO-02/00_architect.md`

## Scope

- In scope:
  - Move the shared aligned metadata byte-read helper out of `boot_sector.rs` into a reusable `io.rs` module.
  - Define a narrow read-only helper API for metadata byte reads at arbitrary volume-byte offsets over `BlockDevice`.
  - Extend `ExfatSuperBlock` with pure geometry helpers derived only from accepted boot geometry.
  - Provide cluster validity predicates and cluster-to-byte or cluster-to-sector translation helpers for later components.
  - Keep this component read-only and independent from `ExfatFs`, inode state, FAT state, page-cache ownership, and mutation policy.
  - Define checker-owned ktest obligations for unaligned metadata reads and cluster-address translation.
- Out of scope:
  - FAT entry semantics, FAT value parsing, and cluster-chain walking.
  - Inode logical-offset to cluster mapping.
  - Page-cache integration or `PageCacheBackend` behavior.
  - `ExfatFs` construction, mount sequencing, or filesystem object ownership.
  - Metadata write helpers, sync helpers, dirty tracking, or writeback policy.
  - Directory-entry parsing, bitmap loading, upcase-table loading, and any namespace behavior.

## Module Specification

- Dependencies:
  - Accepted `EXR-BOOT-01` output in `boot_sector.rs` and `super_block.rs`.
  - `aster_block::{BLOCK_SIZE, BlockDevice}`.
  - `ostd::mm::VmIo`.
  - Existing kernel error conventions under `kernel/`.
- Interfaces provided:
  - `io.rs` owns a reusable aligned metadata byte-read helper:
    - `read_metadata_bytes(block_device: &dyn BlockDevice, offset: usize, buf: &mut [u8]) -> Result<()>`
  - `super_block.rs` owns pure geometry helpers on `ExfatSuperBlock`:
    - `sector_size(&self) -> usize`
    - `cluster_size(&self) -> usize`
    - `cluster_size_in_sectors(&self) -> u32`
    - `cluster_to_byte_offset(&self, cluster: u32) -> Result<usize>`
    - `cluster_to_sector(&self, cluster: u32) -> Result<u64>`
    - `is_valid_cluster(&self, cluster: u32) -> bool`
    - `is_cluster_range_valid(&self, range: Range<u32>) -> bool`
  - `boot_sector.rs` may call the shared `read_metadata_bytes` helper, but must not continue owning a private duplicate read path.
  - `mod.rs` only wires modules and, if needed, reexports helpers for later `exfat_refactor` modules.
- Files/modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- Hidden implementation details:
  - The internal bounce-buffer sizing and exact alignment math used by `read_metadata_bytes`.
  - Any private helper used to convert a cluster identifier into a zero-based data-region index.
  - Any private overflow checks that support translation without widening the public API.

## Functional Specification

### Operation

- Name: `read_metadata_bytes`
- Inputs:
  - `block_device: &dyn BlockDevice`
  - `offset: usize`
  - `buf: &mut [u8]`
- Preconditions:
  - `offset` is a volume-byte offset, not a sector index or cluster-relative offset.
  - `buf` points to writable caller-owned memory.
- Actions:
  - If `buf` is empty, return success without touching the device.
  - Compute the minimal `BLOCK_SIZE`-aligned enclosing byte range that covers `offset..offset + buf.len()`.
  - Read exactly that aligned range from `block_device`.
  - Copy only the requested subrange into `buf`.
  - Perform overflow checks when computing the enclosing range and requested slice bounds.
- Outputs:
  - `Result<()>`
- Postconditions:
  - On success, `buf` contains the exact bytes from the requested volume-byte range.
  - No persistent state, caches, or synchronization objects are created or mutated.
- Error cases:
  - Overflow while computing aligned bounds returns an invalid-argument or invalid-data style error.
  - Any device-read failure propagates as an I/O-style error.
  - Requests that extend beyond what the underlying `BlockDevice` can read propagate the block-device error unchanged.

### Operation

- Name: `ExfatSuperBlock::sector_size`
- Inputs:
  - `&self`
- Preconditions:
  - `self` was created from an accepted `ExfatBootSector`.
- Actions:
  - Return the normalized sector size in bytes as `usize`.
- Outputs:
  - `usize`
- Postconditions:
  - The returned value equals the already-normalized `sector_size` field.
- Error cases:
  - None.

### Operation

- Name: `ExfatSuperBlock::cluster_size`
- Inputs:
  - `&self`
- Preconditions:
  - `self` was created from an accepted `ExfatBootSector`.
- Actions:
  - Return the normalized cluster size in bytes as `usize`.
- Outputs:
  - `usize`
- Postconditions:
  - The returned value equals the already-normalized `cluster_size` field.
- Error cases:
  - None.

### Operation

- Name: `ExfatSuperBlock::cluster_size_in_sectors`
- Inputs:
  - `&self`
- Preconditions:
  - `self` was created from an accepted `ExfatBootSector`.
- Actions:
  - Return the normalized number of sectors per cluster.
- Outputs:
  - `u32`
- Postconditions:
  - The returned value equals `sect_per_cluster`.
- Error cases:
  - None.

### Operation

- Name: `ExfatSuperBlock::is_valid_cluster`
- Inputs:
  - `&self`
  - `cluster: u32`
- Preconditions:
  - None.
- Actions:
  - Compare `cluster` against the accepted exFAT valid data-cluster range.
- Outputs:
  - `bool`
- Postconditions:
  - Returns `true` iff `cluster` is in the inclusive range `EXFAT_RESERVED_CLUSTERS..=num_clusters`.
- Error cases:
  - None.

### Operation

- Name: `ExfatSuperBlock::is_cluster_range_valid`
- Inputs:
  - `&self`
  - `range: Range<u32>`
- Preconditions:
  - `range` uses exFAT cluster identifiers and Rust half-open range semantics.
- Actions:
  - Accept the range only when:
    - `range.start >= EXFAT_RESERVED_CLUSTERS`,
    - `range.end <= num_clusters + 1`,
    - `range.start <= range.end`.
- Outputs:
  - `bool`
- Postconditions:
  - Returns `true` iff every cluster in the half-open range is valid.
- Error cases:
  - None.

### Operation

- Name: `ExfatSuperBlock::cluster_to_byte_offset`
- Inputs:
  - `&self`
  - `cluster: u32`
- Preconditions:
  - `cluster` is expected to denote a data-region cluster identifier.
- Actions:
  - Reject invalid cluster identifiers using `is_valid_cluster`.
  - Convert `cluster` into a zero-based cluster index relative to the data region by subtracting `EXFAT_RESERVED_CLUSTERS`.
  - Multiply that index by `cluster_size`.
  - Add the result to `data_start_sector * sector_size`.
  - Perform checked arithmetic for every multiplication and addition.
- Outputs:
  - `Result<usize>`
- Postconditions:
  - On success, the result is the volume-byte offset of the first byte of the cluster.
  - No FAT walking, chain semantics, or logical file offset policy are involved.
- Error cases:
  - Invalid cluster identifiers return an invalid-argument or invalid-data style error.
  - Arithmetic overflow returns an invalid-data style error.

### Operation

- Name: `ExfatSuperBlock::cluster_to_sector`
- Inputs:
  - `&self`
  - `cluster: u32`
- Preconditions:
  - `cluster` is expected to denote a data-region cluster identifier.
- Actions:
  - Reject invalid cluster identifiers using `is_valid_cluster`.
  - Convert `cluster` into a zero-based cluster index relative to the data region.
  - Multiply that index by `sect_per_cluster`.
  - Add the result to `data_start_sector`.
  - Perform checked arithmetic for every multiplication and addition.
- Outputs:
  - `Result<u64>`
- Postconditions:
  - On success, the result is the first sector index of the cluster within the volume.
  - The result is derived solely from accepted boot geometry, without chain or inode policy.
- Error cases:
  - Invalid cluster identifiers return an invalid-argument or invalid-data style error.
  - Arithmetic overflow returns an invalid-data style error.

## Invariants

- `read_metadata_bytes` is a read-only helper. It never performs writes, syncs, or cache mutation.
- Offset inputs to `read_metadata_bytes` are always interpreted as volume-byte offsets.
- `cluster_to_byte_offset(cluster)` and `cluster_to_sector(cluster)` reject clusters outside the valid data-cluster range.
- For any valid `cluster`, `cluster_to_byte_offset(cluster) / sector_size() == cluster_to_sector(cluster)` as long as `sector_size()` divides the byte offset exactly, which it must for accepted exFAT geometry.
- `cluster_size() == sector_size() * cluster_size_in_sectors()`.
- Translation helpers remain pure with respect to `ExfatSuperBlock`; they do not access the block device, FAT, or page cache.

## Concurrency Specification

- Shared state:
  - `read_metadata_bytes` borrows a shared `BlockDevice`.
  - `ExfatSuperBlock` helpers read immutable normalized geometry only.
- Locking or serialization assumptions:
  - This component introduces no new locks, mutexes, caches, or global mutable state.
  - Callers are responsible for any higher-level serialization required by mount or later mutable components.
- Required atomicity:
  - Each block-device read is atomic only at the underlying block-device interface level.
  - The helper does not promise a snapshot across multiple separate helper calls.
- Forbidden interleavings:
  - The creator must not add shared mutable caches, writeback state, or helper-owned synchronization to this component.
  - The creator must not fold FAT walking, inode mapping, or page-cache coordination into these helpers.
- Behavior under concurrent readers/writers:
  - Concurrent readers are acceptable if the underlying `BlockDevice` supports them.
  - External concurrent writers to the same volume are outside this component's guarantees.

Concurrency work for this component is trivial.
The first modular or functional creator pass may satisfy the full concurrency spec as long as it keeps the component pure and introduces no shared mutable state.
A separate concurrency-only creator pass is not required unless the implementation unexpectedly introduces helper-owned shared state, which should be treated as a design error and surfaced to the main agent.

## Tests and Observability

- Checker-owned unit or kernel tests expected:
  - A checker-owned `#[ktest]` that reads an unaligned boot-region slice crossing a `BLOCK_SIZE` boundary and confirms the returned bytes match the embedded exFAT image.
  - A checker-owned `#[ktest]` that reads the boot checksum sector through `read_metadata_bytes` and confirms exact byte equality with the embedded image.
  - A checker-owned `#[ktest]` that validates `cluster_to_byte_offset` and `cluster_to_sector` against known-good geometry from the embedded exFAT image.
  - A checker-owned `#[ktest]` that confirms invalid cluster identifiers are rejected by translation helpers.
  - A checker-owned `#[ktest]` that confirms `is_cluster_range_valid` honors half-open range semantics and rejects ranges that cross below `EXFAT_RESERVED_CLUSTERS` or above `num_clusters + 1`.
- Observable behaviors the checker should verify:
  - `boot_sector.rs` no longer owns a private aligned metadata-read path.
  - Shared metadata reads return exact bytes for unaligned offsets and lengths.
  - Cluster translation results match accepted `ExfatSuperBlock` geometry and legacy formulas.
  - The component stays read-only and does not grow write or sync semantics.

## Creator Notes

The creator must not silently reinterpret the following constraints:

- Implement only the shared read-side metadata I/O helper and pure geometry helpers in this component.
- Ignore the checker-owned `#[ktest]` obligations above. The creator should not write or update tests in this pass unless the main agent explicitly overrides the workflow.
- Do not add metadata writes, sync helpers, page-cache ownership, `ExfatFs` construction, FAT walking, or inode mapping logic.
- Keep offset units explicit in names, docs, and internal reasoning. Do not mix byte offsets, sector indices, and cluster identifiers.
- Keep helper visibility narrow. If a helper is only needed by `boot_sector.rs` and later internal modules, prefer `pub(super)` over a wider surface.
- If implementation pressure suggests adding non-trivial synchronization or shared mutable caches, stop and surface the issue to the main agent instead of widening the component.
