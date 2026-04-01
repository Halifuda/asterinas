<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: EXR-INOKEY-05A
- Title: Inode Identity Key And Opened-Inode Lookup
- Status: `Specified`
- Author: designer
- Date: 2026-04-01

## Purpose

Define the narrow, creator-facing core for exFAT inode identity: derive a stable inode key from a validated on-disk primary-dentry location and preserve the root special case explicitly.

This component is not a mount component, not an inode lifecycle component, and not a metadata-shaping component. Exact opened-inode lookup is deferred until `EXR-MOUNT-09` owns filesystem-wide state. If the implementation starts needing inode construction, parent tracking, eviction, or registry mutation policy, the boundary is too coarse and must be split instead of widened.

## Scope

- In scope:
  - A single canonical inode-key helper surface for ordinary callers.
  - An explicit root special case that bypasses location packing.
  - Validation that key packing does not silently truncate the byte offset.
  - Minimal module wiring for the new key helpers.
- Out of scope:
  - Inode metadata shaping or persistence.
  - Inode creation, eviction, parent tracking, or deletion policy.
  - Directory iteration, dentry scanning, or file-record parsing.
  - Mount sequencing, filesystem-wide ownership, or any `EXR-MOUNT-09` state object.
  - Page-cache behavior, file I/O, or VFS-facing inode operations.
  - Registry mutation policy beyond whatever external code may need later.

## Prior-Derived Rules To Preserve

These are the local and external rules that later creator or checker work cannot safely infer from the new code alone:

- The legacy Asterinas inode cache keys by on-disk location, not by inode number or cluster alone.
- The legacy key shape is the packed `(cluster << 32) | offset` convention from `utils.rs`.
- The root inode is a reserved special case and uses `ROOT_INODE_HASH = 0`.
- The Linux exFAT implementation summary also keys inode lookup by the on-disk location of the primary directory entry.
- The future filesystem-wide opened-inode lookup owner belongs to `EXR-MOUNT-09`, not this component.

## Module Specification

- Dependencies:
  - `EXR-CHAIN-03B` for validated cluster-walk position data.
  - `EXR-FILESET-04B` for the trusted file-record boundary that later callers will already have.
  - Existing kernel `HashMap`, `Result`, and error conventions.
- Files to touch:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- Canonical interfaces:
  - `ExfatInodeKey` as the stable identity value type.
  - `ExfatInodeKey::from_cluster_and_offset(cluster: ClusterId, byte_offset_in_cluster: usize) -> Result<Self>` as the canonical helper for ordinary callers.
  - `ExfatInodeKey::root() -> Self` as the explicit root special-case constructor.
- Deferred future use:
  - exact opened-inode lookup will eventually use `ExfatInodeKey`, but no standalone lookup wrapper is kept before `EXR-MOUNT-09`.
- Hidden implementation details:
  - Whether `ExfatInodeKey` stores the packed value as a `u64`, `usize`, or a small newtype wrapper.

The creator must keep the helper surface narrow:

- Ordinary callers use the cluster-plus-offset helper.
- Root callers use the explicit root constructor.
- No second ordinary helper may be added unless it solves a different trust boundary.

## Data And Control Flow

- The caller obtains a validated on-disk primary-dentry location from later directory or inode code.
- The caller passes the cluster id and byte offset within the cluster to the canonical key helper.
- The helper validates that the offset fits the packed low 32 bits and returns the stable key.
- Root callers bypass packing and use the explicit root constructor.
- Later mount-owned lookup code may use the key directly once `EXR-MOUNT-09` exists.

## Functional Rules

### Operation

- Name: `ExfatInodeKey::from_cluster_and_offset`
- Inputs:
  - `cluster: ClusterId`
  - `byte_offset_in_cluster: usize`
- Preconditions:
  - The caller already has a validated on-disk primary-dentry location.
  - The cluster id is a real data-cluster id from chain walking or an equivalent trusted source.
- Actions:
  - Pack the key from the cluster id and the byte offset.
  - Preserve the legacy 64-bit layout so existing and future callers derive the same key from the same location.
  - Reject offsets that do not fit the packed low 32-bit field rather than truncating them silently.
- Outputs:
  - `Result<ExfatInodeKey>`
- Error cases:
  - Invalid arguments and offset overflow return invalid-argument style errors.

### Operation

- Name: `ExfatInodeKey::root`
- Inputs:
  - None.
- Actions:
  - Return the reserved identity key for the root inode.
  - Keep the root key distinct from any packed location-derived key.
- Outputs:
  - `ExfatInodeKey`
- Error cases:
  - None.

## Invariants

- The ordinary inode key is derived only from validated on-disk location data.
- The root inode is an explicit reserved case, not a degenerate packed location.
- The packed key shape stays stable across callers.
- The component does not own mount sequencing or inode lifecycle policy.
- The component does not need directory scanning to answer key or lookup questions.

## Creator Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add the key value type and the canonical ordinary constructor.
  - Add the explicit root constructor.
  - Wire the new module(s) through `mod.rs`.
- Explicit non-goals:
  - No inode construction or eviction policy.
  - No parent tracking or child propagation.
  - No mount object.
  - No registry mutation policy.
  - No directory iteration or namespace behavior.

### Serial Checker Pass

- Required checker-owned tests:
  - Key packing round-trip coverage for a valid cluster and byte offset.
  - Explicit root-key coverage.
  - Overflow or truncation rejection coverage for the packed offset field.
- Observable properties that must pass before leaving the serial loop:
  - Ordinary callers use one canonical location-based key helper.
  - Root remains a reserved special case.

## Risks

- If the creator adds a second ordinary key constructor, later callers will have to guess which surface is canonical.
- If the implementation silently truncates the offset field, the key is no longer stable by construction.
- If inode metadata shaping appears, the boundary is wrong; that work belongs in a later inode component.
