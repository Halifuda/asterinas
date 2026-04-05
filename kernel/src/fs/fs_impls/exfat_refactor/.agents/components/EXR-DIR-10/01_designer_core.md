<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-DIR-10`
- Title: Directory Iteration And Lookup Over Shared Filesystem State
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-05`
- Task packet: `EXR-DIR-10-DESIGN-20260405-1058`
- Based on architect artifact: `00_architect.md`

## Scope

- In scope:
  - Read-only directory iteration over already-mounted exFAT state.
  - Name lookup against validated directory/file-record surfaces.
  - Canonical case-insensitive matching through the loaded upcase-table service.
  - One directory-owned entry point that can serve lookup and iteration without rediscovering mount state.
  - Root-directory and subdirectory traversal as consumers of mounted shared state.
- Out of scope:
  - Mount bootstrap, root discovery, or any second mount path.
  - Page-cache-backed regular-file reads or buffered data I/O.
  - Namespace mutation, including `create`, `unlink`, `mkdir`, `rmdir`, and `rename`.
  - Allocation policy, bitmap mutation, or FAT-chain growth.
  - A second overlapping lookup helper or a separate comparison policy surface.

## Module Specification

- Dependencies:
  - `EXR-MOUNT-09`
  - `EXR-FILESET-04B`
  - `EXR-INODE-05B`
  - `EXR-UPCASE-07B`
  - `EXR-DENTRY-04A`
- Interfaces provided:
  - One canonical directory entry point in `dir.rs` that accepts a validated directory inode shell plus a read-only request describing either lookup or iteration.
  - One private directory-walking implementation that scans validated directory records and reuses the same candidate-filter logic for lookup and iteration.
  - No additional public helper for name folding, raw comparison, or mount discovery.
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/dir.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- Hidden implementation details:
  - Whether the canonical entry point returns a cursor-oriented iterator item, a lookup result, or both through a request enum.
  - Whether lookup and iteration share one private scanner or one private scanner plus a small request-specific adapter.
  - Whether the directory walk keeps its current position as a cluster/entry offset or as a validated cursor wrapper.

The canonical surface must stay narrow. Later code should call one directory-owned entry point instead of a separate lookup helper, a separate iteration helper, or any mount-discovery path.

## Functional Specification

### Operation

- Name: canonical directory lookup and iteration
- Inputs:
  - A borrowed mount-owned filesystem object published by `EXR-MOUNT-09`.
  - A validated directory inode shell from `EXR-INODE-05B`.
  - A read-only request that identifies either name lookup or directory iteration.
  - For lookup, a logical UTF-16 name slice already normalized by the caller's external path decoding layer.
- Preconditions:
  - The filesystem object is already published and immutable from this component's point of view.
  - The caller is not asking this component to rediscover the upcase table, the root directory, or any allocation state.
  - The caller is not asking this component to mutate namespace metadata or regular-file contents.
  - The directory inode shell already represents a validated directory, not an ordinary file.
- Actions:
  - Walk directory entries from validated directory/file-record surfaces only.
  - Treat `0x00` as end-of-directory and stop scanning immediately.
  - Skip deleted or free entries linearly.
  - For lookup, use the stream-entry `name_len` and `name_hash` as the fast filter before any full comparison.
  - Compare candidate names through the canonical upcase-backed service, not through a raw UTF-16 or mount-local fallback path.
  - Reuse the same candidate-scanning policy for iteration and lookup so the entry-order and comparison rules cannot drift apart.
- Outputs:
  - For lookup, the matched validated directory/file-record surface or an equivalent read-only representation that later VFS code can consume.
  - For iteration, the next validated directory/file-record surface or end-of-directory.
- Postconditions:
  - The result reflects the canonical exFAT case-insensitive rules.
  - No mutation, writeback, or mount discovery has occurred.
  - Directory traversal stops as soon as the on-disk directory says it is done.

## Invariants

- Directory iteration consumes mounted shared state; it does not create it.
- The canonical upcase-table service is the only name-normalization authority for this component.
- Lookup and iteration share one candidate-walking policy.
- The component does not own namespace mutation, page-cache state, or file-data reads.
- Validated file-record surfaces from `EXR-FILESET-04B` remain trusted boundaries during scanning.
- The component never needs to rediscover the root directory or the upcase table from disk.
- The same read-only directory scanner must work for root and non-root directories.

## Concurrency Specification

- Shared state:
  - One borrowed mount-owned filesystem object.
  - Immutable validated directory metadata and candidate file-record surfaces during a scan.
- Lock ordering:
  - None inside this component's contract.
- Atomicity requirements:
  - A lookup or iteration pass must observe a stable, already-published mount state.
  - A candidate record is either fully validated against the canonical name rules or rejected; no partially matched result is published.
- Forbidden interleavings:
  - No mount bootstrap interleaving with directory lookup.
  - No page-cache or buffered read interleaving.
  - No namespace mutation interleaving.
- Allowed simplifications such as a temporary big lock:
  - No dedicated lock is required in this component.
  - The only serialization assumption is that `EXR-MOUNT-09` has already published the shared filesystem object before directory scanning begins.

No separate async artifact is needed because this component is synchronous, read-only, and has no awaitable contract or shared mutable state. The residual serialization assumption is the one stated above: lookup and iteration only run after mount publication.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add one canonical directory-owned entry point in `dir.rs`.
  - Implement directory iteration over validated directory records from mounted shared state.
  - Implement lookup filtering with the canonical upcase-backed name-hash and comparison path.
  - Keep the lookup and iteration policy together so the two paths cannot diverge.
  - Preserve the boundary between lookup and mutation.
- Explicit non-goals:
  - No create, unlink, mkdir, rmdir, or rename behavior.
  - No regular-file reads, page-cache behavior, or data-path buffering.
  - No second public helper for name comparison or mount discovery.
  - No async tasks, channels, atomics, or background coordination.

### Serial Checker Pass

- Required checker-owned tests:
  - A lookup regression that proves mixed-case input matches the same directory entry through the canonical upcase-backed path.
  - A lookup regression that proves a name-hash mismatch is rejected before a candidate is accepted.
  - A directory-iteration regression that proves scanning stops at `0x00` end-of-directory and skips deleted or free entries linearly.
  - A root-versus-subdirectory regression that proves the same canonical entry point can read both kinds of validated directory state from the published mount anchor.
- Observable properties that must pass before leaving the serial loop:
  - Case-insensitive lookup follows the canonical table-backed service.
  - Iteration and lookup share one candidate policy.
  - The component does not drift into namespace mutation or file-data reads.
  - The tests stay local and do not require async harnesses.

### Concurrency Creator Pass

- Required implementation obligations:
  - None beyond the read-only shared-state boundary already recorded above.
- Explicit non-goals:
  - No concurrent publication protocol beyond the mount handoff.
  - No shared mutable cache ownership in this component.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - None; the read-only lookup-and-iteration boundary is satisfied by the serial regressions and the mount publication contract.

## Acceptance Notes

- Reviewer should verify that the final surface is still one canonical directory entry point rather than separate lookup and iteration helpers.
- Reviewer should reject any attempt to fold in create, unlink, mkdir, rmdir, rename, or regular-file read behavior.
- Reviewer should pay attention to whether name matching still uses the upcase-backed canonical service and not a second local comparison rule.
- If a helper surface appears, it should have a named downstream caller; otherwise it should not exist.
