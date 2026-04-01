<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Specification

## Metadata

- Component ID: EXR-CHAIN-03B
- Title: Chain State And Read-Only Cluster Walking
- Status: `Specified`
- Author: designer
- Date: 2026-04-01
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHAIN-03B/00_architect.md`

## Scope

- In scope:
  - Define a small chain-state type that records the current cluster, the remaining cluster count, and whether the chain is contiguous or FAT-backed.
  - Provide one constructor path that accepts either a known length or an unknown length, counting from the chain head only when FAT traversal is available.
  - Provide read-only cluster-walking helpers for contiguous and FAT-backed chains.
  - Provide offset-to-chain-position helpers for later read mapping.
  - Keep empty-chain semantics explicit so later write-side work can build on the same API without guessing.
  - Add checker-owned tests for contiguous traversal, FAT-backed traversal, unknown-length counting, empty-chain handling, and invalid-step rejection.
- Out of scope:
  - Allocation, extension, free, truncation, or bitmap mutation logic.
  - Any write-back or dirty-state management for FAT entries.
  - `ExfatFs` ownership, mount policy, inode policy, or page-cache coordination.
  - Namespace, directory-entry parsing, or read-mapping beyond chain position helpers.
  - Backup-chain recovery, allocation fallback, or any later write-side chain consumers.

## Module Specification

- Dependencies:
  - `EXR-BOOT-01` normalized geometry through `ExfatSuperBlock`.
  - `EXR-IO-02` shared metadata-byte reads and cluster translation helpers.
  - `EXR-FATVAL-03A` typed FAT values and single-step next-cluster decode.
  - Existing kernel error conventions for invalid arguments and I/O failures.
- Interfaces provided:
  - `ClusterId = u32`
  - `ChainMode` enum with:
    - `Contiguous`
    - `FatBacked`
  - `ExfatChain` state with:
    - current cluster,
    - remaining cluster count from the current position, inclusive,
    - chain mode.
  - Read-only helpers on `ExfatChain` for:
    - construction,
    - accessors,
    - walking by cluster steps,
    - walking to a byte offset within the chain,
    - translating the current cluster to a physical byte offset.
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- Hidden implementation details:
  - Whether the chain-counting helper is private or split into a small private count-and-validate routine plus a public constructor.
  - The exact internal representation of `ChainMode`.
  - The private arithmetic helpers used to validate contiguous cluster increments and offset division.

The creator must keep this module narrow:

- `fat.rs` owns the chain state and read-only walking logic.
- `read_next_fat_value` remains the only FAT-entry decode helper used by chain traversal.
- Allocation and truncation behavior must stay out of this component even if the legacy file still contains those concerns elsewhere.

## Functional Specification

### Operation

- Name: `ExfatChain::new`
- Inputs:
  - `block_device: &dyn BlockDevice`
  - `super_block: &ExfatSuperBlock`
  - `current: ClusterId`
  - `num_clusters: Option<u32>`
  - `mode: ChainMode`
- Preconditions:
  - `super_block` is already validated and normalized.
  - `current == 0` is only used for an empty chain.
  - If `num_clusters` is `None`, the chain must be FAT-backed or empty.
- Actions:
  - Accept an explicit cluster count as-is when provided.
  - If the length is unknown and the chain is FAT-backed, count clusters from `current` by repeatedly reading next FAT values until `EndOfChain`.
  - Include the head cluster in the count.
  - Reject malformed FAT chains encountered during counting, including:
    - invalid source clusters,
    - invalid decoded next-cluster targets,
    - `Free` or `Bad` entries encountered before the end of the chain,
    - a missing terminal `EndOfChain`.
  - Reject unknown-length contiguous chains, because they have no FAT-backed counting path.
  - Accept the empty chain without reading the FAT.
- Outputs:
  - `Result<ExfatChain>`
- Postconditions:
  - The returned chain stores the current cluster, remaining cluster count, and mode only.
  - No mutable filesystem state is created or modified.
- Error cases:
  - Invalid arguments and out-of-range clusters return invalid-argument style errors.
  - Malformed FAT traversal returns I/O-style errors.

### Operation

- Name: `ExfatChain::current_cluster`
- Inputs:
  - `&self`
- Actions:
  - Return the current cluster identifier.
- Outputs:
  - `ClusterId`
- Postconditions:
  - The accessor is pure.
- Error cases:
  - None.

### Operation

- Name: `ExfatChain::cluster_count`
- Inputs:
  - `&self`
- Actions:
  - Return the remaining cluster count from the current position, inclusive.
- Outputs:
  - `u32`
- Postconditions:
  - A count of `0` means the chain is empty.
- Error cases:
  - None.

### Operation

- Name: `ExfatChain::mode`
- Inputs:
  - `&self`
- Actions:
  - Return the chain traversal mode.
- Outputs:
  - `ChainMode`
- Postconditions:
  - The accessor is pure.
- Error cases:
  - None.

### Operation

- Name: `ExfatChain::is_empty`
- Inputs:
  - `&self`
- Actions:
  - Return whether the chain contains no clusters.
- Outputs:
  - `bool`
- Postconditions:
  - Empty chains use `current == 0` and `cluster_count == 0`.
- Error cases:
  - None.

### Operation

- Name: `ExfatChain::walk`
- Inputs:
  - `&self`
  - `block_device: &dyn BlockDevice`
  - `super_block: &ExfatSuperBlock`
  - `steps: u32`
- Preconditions:
  - The chain is non-empty.
  - `steps` counts cluster hops from the current cluster, not bytes.
- Actions:
  - Reject any walk on an empty chain.
  - Reject any step count greater than or equal to the remaining cluster count.
  - For contiguous chains:
    - compute the destination cluster by checked addition from the current cluster,
    - reject arithmetic overflow or any resulting invalid cluster identifier,
    - do not read the FAT.
  - For FAT-backed chains:
    - follow exactly `steps` successive `Next` links using `read_next_fat_value`,
    - reject any non-`Next` marker before the final hop,
    - reject malformed decode results from the FAT helper.
  - Return a new chain whose current cluster is the destination cluster and whose remaining count is reduced by `steps`.
- Outputs:
  - `Result<ExfatChain>`
- Postconditions:
  - The walk is read-only.
  - The destination chain preserves the original mode.
- Error cases:
  - Invalid step counts return invalid-argument style errors.
  - Overflow, invalid destination clusters, and malformed FAT traversal return errors instead of wrapping.

### Operation

- Name: `ExfatChain::walk_to_cluster_at_offset`
- Inputs:
  - `&self`
  - `block_device: &dyn BlockDevice`
  - `super_block: &ExfatSuperBlock`
  - `offset: usize`
- Preconditions:
  - `super_block.cluster_size()` is a positive power-of-two byte size.
- Actions:
  - Compute the number of whole-cluster steps as `offset / cluster_size`.
  - Compute the intra-cluster byte offset as `offset % cluster_size`.
  - Delegate cluster movement to `walk`.
  - Reject offsets that land beyond the chain end.
- Outputs:
  - `Result<(ExfatChain, usize)>`
- Postconditions:
  - The returned chain points at the cluster containing the requested byte offset.
  - The second tuple element is always the byte offset within that cluster.
- Error cases:
  - Out-of-range offsets propagate the same invalid-step or malformed-chain error path as `walk`.

### Operation

- Name: `ExfatChain::physical_cluster_start_offset`
- Inputs:
  - `&self`
  - `super_block: &ExfatSuperBlock`
- Preconditions:
  - The chain is non-empty.
- Actions:
  - Translate the current cluster to its physical byte offset using `super_block`.
- Outputs:
  - `Result<usize>`
- Postconditions:
  - The returned byte offset is the first byte of the current cluster.
- Error cases:
  - Empty chains and invalid clusters return errors.

## Invariants

- `cluster_count == 0` means the chain is empty.
- Empty chains use `current == 0`.
- Non-empty chains always have a valid data-region `current` cluster.
- The stored `cluster_count` is inclusive of the current cluster.
- `walk(0)` is a no-op on a non-empty chain.
- `walk(steps)` never returns a chain whose current cluster falls outside the valid data-cluster range.
- Contiguous walking never reads the FAT.
- FAT-backed walking depends only on `read_next_fat_value` and never re-parses raw FAT bytes inline.
- `read_next_fat_value` must continue to reject invalid decoded next-cluster targets before chain traversal uses them.
- No allocation, free, truncation, or bitmap mutation logic lives in this component.

## Concurrency Specification

- Shared state:
  - borrowed `BlockDevice`
  - immutable `ExfatSuperBlock`
- Locking or serialization assumptions:
  - This component introduces no locks, mutexes, caches, or global mutable state.
  - Callers are responsible for any higher-level serialization needed to keep the underlying volume stable.
- Required atomicity:
  - Each individual device read is only as atomic as the block-device interface beneath it.
  - Multi-hop FAT-backed walks are not snapshot-atomic across the whole traversal.
- Forbidden interleavings:
  - Do not add helper-owned shared state, writeback state, or allocation state here.
  - Do not make the chain helpers depend on mount-wide ownership or inode locks.
- Behavior under concurrent readers or writers:
  - Concurrent readers are acceptable if the block device supports them.
  - Concurrent external writers are outside this component's guarantees and may surface as traversal errors.

## Tests and Observability

- Checker-owned unit or kernel tests expected:
  - A `#[ktest]` that constructs a contiguous chain with an explicit length and verifies that `walk` advances the current cluster without consulting FAT state.
  - A `#[ktest]` that constructs a FAT-backed chain from the embedded exFAT image and verifies `walk` follows the decoded next-cluster target.
  - A `#[ktest]` that constructs a FAT-backed chain with an unknown length and verifies the constructor counts the chain from the head.
  - A `#[ktest]` that confirms a malformed FAT-backed chain is rejected during counting when the chain contains a non-terminating invalid marker.
  - A `#[ktest]` that confirms `walk` rejects an invalid step count, including stepping past the end of the chain.
  - A `#[ktest]` that confirms the empty-chain constructor path is explicit and that traversal on an empty chain is rejected.
- Observable behaviors the checker should verify:
  - Contiguous traversal is pure arithmetic over cluster IDs.
  - FAT-backed traversal depends on the validated FAT decoder, not raw entry interpretation.
  - Unknown-length construction is supported only where FAT traversal can supply the count.
  - The component remains read-only and introduces no allocation or bitmap activity.

## Creator Notes

The creator must not silently widen this component into later chain-management work.

- Keep allocation, free, truncation, and bitmap mutation out of `EXR-CHAIN-03B`.
- Keep the chain state minimal and read-only.
- Keep empty-chain semantics explicit instead of folding them into ad hoc traversal behavior.
- If the implementation pressure starts pushing this beyond a small read-only chain slice, stop and report that the architect scope is too large rather than widening the task board.
