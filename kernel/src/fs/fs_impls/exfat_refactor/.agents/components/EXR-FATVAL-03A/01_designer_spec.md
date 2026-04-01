<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Specification

## Metadata

- Component ID: EXR-FATVAL-03A
- Title: FAT Entry Value Model And Single-Step Next-Cluster Decode
- Status: `Specified`
- Author: main-agent
- Date: 2026-04-01
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FATVAL-03A/00_architect.md`

## Scope

- In scope:
  - Define a typed FAT entry value enum for `free`, `bad`, `end-of-chain`, and `next-cluster`.
  - Define raw `u32` to typed FAT-value conversion and the reverse conversion for testability and later write-side work.
  - Provide one read-only helper that reads and decodes the FAT entry for a single cluster from the first FAT.
  - Reject invalid source cluster identifiers and invalid decoded next-cluster targets.
  - Add checker-owned ktests for raw decoding and at least one on-disk decode path.
- Out of scope:
  - Cluster-chain traversal across multiple hops.
  - Counting chain length.
  - Contiguous-chain optimization flags.
  - Allocation, free, truncate, or bitmap mutation logic.
  - FAT writeback or dirty-state management.

## Module Specification

- Dependencies:
  - `EXR-IO-02` read-side metadata helper and cluster validation geometry.
  - `EXR-BOOT-01` constants for first and reserved cluster ranges.
- Interfaces provided:
  - `ClusterId = u32`
  - `FatValue` enum with:
    - `Free`
    - `Next(ClusterId)`
    - `Bad`
    - `EndOfChain`
  - `impl From<ClusterId> for FatValue` or an equivalent narrow decoder helper, as long as invalid next-cluster targets are not silently accepted by the on-disk read helper.
  - `impl From<FatValue> for ClusterId` for reversible special-value encoding.
  - `read_next_fat_value(block_device: &dyn BlockDevice, super_block: &ExfatSuperBlock, cluster: ClusterId) -> Result<FatValue>`
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- Hidden implementation details:
  - private helpers for FAT-entry byte-offset calculation,
  - private validation helper for decoded next-cluster targets.

## Functional Specification

- `read_next_fat_value`:
  - accepts a validated data-region cluster identifier,
  - computes the FAT-entry byte offset as `fat1_start_sector * sector_size + cluster * 4`,
  - reads exactly one little-endian `u32` FAT entry through `read_metadata_bytes`,
  - decodes the raw value into `FatValue`,
  - returns an error if:
    - the source cluster is not a valid cluster,
    - the decoded `Next(next_cluster)` target is not a valid cluster.
- Raw-value mapping rules:
  - `0` => `Free`
  - `0xFFFF_FFF7` => `Bad`
  - `0xFFFF_FFFF` => `EndOfChain`
  - any other raw value => `Next(raw_value)`
- Later chain components may rely on:
  - `Bad`, `Free`, and `EndOfChain` being losslessly distinguished from `Next`.
  - `read_next_fat_value` never returning `Next` with an invalid cluster target.

## Invariants

- `read_next_fat_value` is read-only.
- Special marker values remain distinct and reversible.
- Invalid source clusters never reach the device-read stage.
- Invalid decoded next-cluster targets are surfaced as errors rather than accepted as ordinary `Next` values.

## Concurrency Specification

- Shared state:
  - borrowed `BlockDevice`
  - immutable `ExfatSuperBlock`
- Lock ordering:
  - none
- Atomicity requirements:
  - one FAT entry read is only as atomic as the block-device read beneath it
- Forbidden interleavings:
  - no helper-owned caches, writeback state, or shared mutable global state
- Allowed simplifications such as a temporary big lock:
  - no dedicated concurrency work is required for this component

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - add `fat.rs`,
  - wire it into `mod.rs`,
  - define the typed value model and read-only decode helper,
  - keep all APIs narrowly read-side.
- Explicit non-goals:
  - no chain walking,
  - no allocation or write helpers,
  - no filesystem object ownership.

### Serial Checker Pass

- Required checker-owned tests:
  - a small pure ktest or unit-style kernel test for special-value decode and reverse conversion,
  - a targeted on-disk ktest that reads at least one FAT entry from the embedded exFAT image,
  - a negative-path test showing invalid source or invalid next-cluster targets are rejected.
- Observable properties that must pass before leaving the serial loop:
  - the typed value model matches the specified raw-value mapping,
  - on-disk reads use the shared metadata I/O helper,
  - invalid clusters do not silently decode as usable successors.

### Concurrency Creator Pass

- Required implementation obligations:
  - no dedicated concurrency implementation required
- Explicit non-goals:
  - do not invent locks or caches for this component

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - the component remains pure and read-only.

## Acceptance Notes

- Reviewer should check that the API surface stays below chain-level semantics.
- Reviewer should also verify that invalid next-cluster targets are not accidentally accepted via a too-permissive raw decoder.
