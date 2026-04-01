<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: EXR-INODE-05B
- Title: Read-Only Inode Metadata Shell
- Status: `Specified`
- Author: designer
- Date: 2026-04-01

## Purpose

Define the smallest implementable inode shell that preserves read-only metadata from validated file-record facts, validated chain facts, and accepted inode identity.

This component is intentionally not the live inode object. It does not own `PageCache`, `PageCacheBackend`, buffered I/O, page-cache sizing, mount sequencing, directory iteration, child counts, parent propagation, registry mutation, or VFS inode behavior.

The root inode remains an explicit synthetic special case. It must not be routed through the ordinary parsed-file-record constructor.

## Scope

- In scope:
  - A read-only inode metadata shell type, working name `ExfatInodeMeta`.
  - One canonical ordinary constructor from accepted inode identity, validated file-record facts, and validated chain facts.
  - One explicit synthetic root constructor.
  - Preservation of validated metadata and chain facts inside the shell without speculative getter surface.
  - Preservation of the file-versus-directory distinction from the validated file record.
- Out of scope:
  - `PageCacheBackend`, `PageCache`, buffered I/O, or cache-size coordination.
  - Read/write entry points, `read_at`, `write_at`, or direct I/O helpers.
  - Directory iteration, lookup, rename, unlink, create, or any other VFS-facing inode operation.
  - Registry mutation, opened-inode insertion/removal, or parent tracking.
  - Mount sequencing or filesystem-wide ownership.

## Prior-Derived Rules To Preserve

These are the rules later creator and checker work must keep explicit:

- Microsoft exFAT file records carry the metadata this shell must preserve: file attributes and timestamps in the `0x85` entry, and `NoFatChain`, name length/hash, valid data length, first cluster, and data length in the `0xC0` entry.
- Microsoft exFAT requires directories to keep `ValidDataLength == DataLength`.
- The `NoFatChain` bit is a placement and traversal fact, not a page-cache policy.
- Linux exFAT keeps inode metadata separate from page-cache behavior and file I/O, even though the legacy implementation combines them in one `inode.c`.
- Asterinas-local constraints require safe Rust under `kernel/`, and page-cache ownership must remain fenced inside `EXR-PGCACHE-11B`.

## Module Specification

- Dependencies:
  - `EXR-CHAIN-03B`
  - `EXR-FILESET-04B`
  - `EXR-INOKEY-05A`
- Files to touch:
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- Canonical interfaces:
  - `ExfatInodeMeta::new(...)` as the ordinary constructor.
  - `ExfatInodeMeta::new_root(...)` as the explicit synthetic root constructor.
- Hidden implementation details:
  - Whether the shell stores a private normalized payload struct or keeps the validated facts directly in the shell type.
  - Whether the timestamps stay in their decoded exFAT representation or are normalized into a local metadata timestamp type.

The creator must keep the API narrow:

- Ordinary callers use `new(...)` with a validated file-record boundary and validated chain facts.
- Root callers use `new_root(...)` and must never pass through the ordinary constructor.
- The shell may store private facts internally, but no getter or field-exposing helper should appear unless a downstream component already proves it needs that boundary.

## Data And Control Flow

- The caller already has an accepted inode identity from `EXR-INOKEY-05A`.
- The caller already has a validated file-record boundary from `EXR-FILESET-04B`.
- The caller already has validated chain facts from `EXR-CHAIN-03B`.
- The ordinary constructor extracts file attributes, timestamps, raw name data, and stream size/cluster facts from the validated file record.
- The ordinary constructor stores the provided chain facts without re-walking the FAT or inventing page-cache state.
- The root constructor accepts a synthetic metadata bundle instead of a parsed file record.
- This component does not need a speculative getter surface yet; later components may request targeted helpers only when a concrete cross-module caller exists.

## Functional Rules

### Operation

- Name: `ExfatInodeMeta::new`
- Inputs:
  - `inode_key: ExfatInodeKey`
  - `file_record: &ExfatDentrySet`
  - `chain: ExfatChain`
- Preconditions:
  - `inode_key` is already accepted by `EXR-INOKEY-05A`.
  - `file_record` already passed the `EXR-FILESET-04B` validation boundary.
  - `chain` already passed the `EXR-CHAIN-03B` validation boundary.
  - The caller is not trying to construct the root inode through this path.
- Actions:
  - Extract and preserve the file-record metadata facts from the validated set.
  - Preserve the raw name payload exactly as exposed by the validated file-record boundary.
  - Store the chain facts unchanged.
  - Reject a directory record whose valid-data length and data length differ.
  - Reject an attempt to use the root key through the ordinary constructor.
- Outputs:
  - `Result<ExfatInodeMeta>`
- Postconditions:
  - The returned shell is read-only and value-like.
  - No page-cache, mount, or registry state is created or modified.

### Operation

- Name: `ExfatInodeMeta::new_root`
- Inputs:
  - `inode_key: ExfatInodeKey`
  - `chain: ExfatChain`
  - `valid_data_length: usize`
  - `data_length: usize`
  - `created_at: DosTimestamp`
  - `modified_at: DosTimestamp`
  - `accessed_at: DosTimestamp`
- Preconditions:
  - `inode_key` is the reserved root key.
  - `chain` is the validated root chain.
  - The supplied root metadata is synthetic, not parsed from a file record.
- Actions:
  - Build the root shell directly from synthetic metadata facts.
  - Force the root shell to behave as a directory shell.
  - Preserve the caller-supplied timestamps and sizes.
  - Reject a synthetic root payload whose valid-data length and data length differ.
- Outputs:
  - `Result<ExfatInodeMeta>`
- Postconditions:
  - The root shell remains an explicit special case.
  - No directory parsing, mount sequencing, or page-cache behavior is introduced.

## Invariants

- `EXR-INODE-05B` stores metadata only.
- The shell does not own page-cache state, buffered I/O state, or VFS operation dispatch.
- The root special case is explicit and cannot be produced accidentally by the ordinary constructor.
- Directory shells must satisfy `valid_data_length == data_length`.
- The chain mode remains a read-only fact from `EXR-CHAIN-03B`; this component does not reinterpret `NoFatChain`.
- Raw name data is preserved structurally, not canonicalized.
- No child count, parent hash, or registry identity is stored here.
- No helper surface is added until a later component proves which facts must cross the module boundary.

## Creator Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add the metadata shell type and keep it confined to `inode.rs`.
  - Implement one ordinary constructor that consumes accepted inode identity, a validated file-record boundary, and validated chain facts.
  - Implement one explicit root constructor for the synthetic root case.
  - Keep stored metadata private until a downstream component proves which helper boundary is needed.
  - Preserve raw name data exactly as provided by the validated file-record boundary.
- Explicit non-goals:
  - No `PageCache`, `PageCacheBackend`, or buffered I/O.
  - No `read_at`, `write_at`, `resize`, or other live inode behavior.
  - No directory traversal, child counting, parent propagation, or registry mutation.
  - No mount-object ownership or filesystem-wide state.

### Serial Checker Pass

- Required checker-owned tests:
  - Validate ordinary construction from a synthetic but already-validated file-record boundary and chain facts.
  - Validate the root special case through `new_root(...)`.
  - Validate that the ordinary constructor rejects the reserved root key.
  - Validate that directory shells reject `valid_data_length != data_length`.
  - Validate that synthetic root payloads reject `valid_data_length != data_length`.
- Observable properties that must pass before leaving the serial loop:
  - The shell remains read-only.
  - The ordinary constructor does not absorb root handling.
  - The root constructor does not require parsed file-record facts.
  - No page-cache or VFS behavior is needed to use the shell.

## Acceptance Notes

- The creator should not be tempted to add a `Metadata`/`stat` adapter here if it drags in VFS policy. That belongs to a later component.
- If the implementation starts needing child counts, parent tracking, or lookup mutation, the boundary has drifted into directory or namespace work and must be split.
- If the implementation starts needing page-cache fields or file I/O hooks, `EXR-PGCACHE-11B` has been pulled in too early.
- The only special-case escape hatch in this component is the synthetic root constructor.
