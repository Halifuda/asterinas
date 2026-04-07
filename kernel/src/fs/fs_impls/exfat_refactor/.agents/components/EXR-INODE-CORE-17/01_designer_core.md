<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: EXR-INODE-CORE-17
- Title: Inode Carrier And Metadata Owner
- Status: `Specified`
- Author: designer
- Date: 2026-04-07
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CORE-17/20260407-1035-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/00_architect.md`

## Scope

- In scope:
  - Define `ExfatInode` as the stable VFS inode carrier for exFAT metadata ownership.
  - Store a weak back-reference to `ExfatFs` and copy trusted inode facts into owner-private scalar state.
  - Expose the VFS identity and metadata surface that is meaningful before inode cache, directory ops, read/write, page cache, or namespace mutation land.
  - Keep `InodeKey`, opened-inode table behavior, and page-cache ownership out of this unit.
- Out of scope:
  - Inode cache and `InodeKey`.
  - Opened-inode table management.
  - Directory lookup, readdir, create, link, unlink, rename, and namespace mutation.
  - Read/write data-path behavior, page cache backend behavior, and sync ordering.
  - Any filesystem-global mount/open sequencing beyond the `fs()` back-reference needed by the carrier.

## Module Specification

- Dependencies:
  - `EXR-FS-CORE-16` filesystem owner boundary.
  - `EXR-FILESET-04B` validated `ExfatDentrySet` inputs.
  - `EXR-CHAIN-03B` validated `ExfatChain` inputs.
  - VFS `Inode`, `InodeIo`, and `FileSystem` contracts.
  - The accepted `EXR-FS-CORE-16` root-handoff seam.
- Interfaces provided:
  - A crate-local `ExfatInode` type that implements VFS `Inode` and `InodeIo`.
  - A weak owner back-reference to `ExfatFs` that can be upgraded on demand for `fs()`.
  - Metadata accessors that reflect the inode snapshot without re-reading the original dentry or chain containers.
- Files or modules touched:
  - Primary landing: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`.
  - Module wiring may require `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`; if so, that declaration edit must be serialized with the sibling `EXR-FS-CORE-16` lane because it is a shared collision point.
- Hidden implementation details:
  - Do not retain `ExfatDentrySet` or `ExfatChain` as surrogate owners.
  - Store only the scalar inode facts copied from those trusted inputs.
  - Keep any constructor or helper surface crate-private unless a later caller is named in this packet.
  - Do not invent field-exposing accessors for `InodeKey` or cache integration.

## Functional Specification

### Construction

- Inputs:
  - A weak reference to the owning `ExfatFs`.
  - Trusted inode identity facts extracted from a validated `ExfatDentrySet`.
  - Trusted chain facts extracted from a validated `ExfatChain`.
  - Trusted filesystem-geometry facts needed to derive size and metadata snapshots.
- Actions:
  - Copy the needed scalar facts into `ExfatInode` at construction time.
  - Drop the dentry-set and chain containers as sources of truth after extraction.
  - Keep the filesystem back-reference weak so the inode does not form a strong reference cycle.
- Outputs:
  - A self-contained inode owner that can satisfy VFS metadata queries without cache or data-path ownership.

### State

`ExfatInode` should own only the facts required to answer the current VFS metadata contract and to anchor later filesystem-owned behavior:

- `Weak<ExfatFs>` back-reference.
- Stable inode identity.
- Inode type and permission mode.
- Owner and group snapshot values.
- Access, modify, and metadata-change timestamps.
- Logical file size and any derived allocation-size snapshot needed to keep `Metadata` coherent.
- Copied chain identity facts such as starting cluster, cluster count, and chain traversal mode.
- Copied dentry-location facts needed for later owner-private identity work, if the constructor already has them.

The type should not store the original dentry set, the original chain object, or any cache/table handle in this unit.

### Metadata Surface

The following methods are meaningful now and should be backed directly by the copied snapshot state:

- `ino()`
- `size()`
- `metadata()`
- `type_()`
- `mode()`
- `owner()`
- `group()`
- `atime()`
- `mtime()`
- `ctime()`
- `fs()`

`metadata()` should synthesize a coherent `Metadata` snapshot from the same scalar fields used by the individual accessors.
The size, mode, ownership, and timestamps reported through `metadata()` must match the dedicated accessors.

`fs()` should upgrade the weak back-reference and return the owning filesystem.
The inode does not own a strong filesystem cycle, so a failed upgrade is a logic error rather than a recoverable state.

### Explicit Temporary Seams

The following methods remain temporary seams until later owner components land:

- `InodeIo::read_at`
- `InodeIo::write_at`
- `resize()`
- `set_mode()`
- `set_owner()`
- `set_group()`

These methods should reject explicitly rather than pretending to be durable behavior.
Use a named temporary seam comment that points to the future owner component, and keep the rejection behavior obvious to later readers.

Recommended seam comment for the data-path methods:

```rust
// Temporary seam: EXR-READ-OPS-25, EXR-WRITE-30, and EXR-PGCACHE-26 will own this path.
```

The setters must not mutate hidden writeback state in this component.
If they are present before namespace and write-side ownership exists, they should reject until later work owns the persistence policy.

### Default Rejections

The remaining directory, xattr, and sync methods should stay on their inherited default rejection path unless a later unit explicitly claims them.
Do not introduce partial directory semantics, partial xattr support, or page-cache shims in this component.

## Invariants

- The inode is a stable VFS carrier, not a cache key and not a file-record wrapper.
- The weak filesystem back-reference is the only filesystem ownership edge from `ExfatInode` to `ExfatFs`.
- `ExfatDentrySet` and `ExfatChain` are trusted inputs, not persistent owners.
- `metadata()` must stay internally consistent with the individual metadata accessors.
- The carrier must not depend on inode-cache state to answer identity or metadata queries.
- Any later directory-name or data-path ownership belongs to future components, not this one.

## Concurrency Specification

- Shared state:
  - No shared mutable state is introduced here beyond the weak filesystem reference and the snapshot fields owned by the inode itself.
- Lock ordering:
  - None is introduced by this component.
- Atomicity requirements:
  - Construction must copy a consistent snapshot from trusted inputs before the original containers are discarded.
  - After construction, metadata accessors are read-only snapshot reads.
- Forbidden interleavings:
  - Do not mutate through the temporary seams while also relying on the snapshot as if it were durable write-side state.
  - Do not add hidden locking or atomic writeback policy inside the inode carrier.
- Allowed simplifications such as a temporary big lock:
  - None required here.

No separate `02_designer_async.md` is needed because this component does not define independent lock-ordering, async, or shared-mutable-state behavior.
Any residual sequencing assumption is recorded above: the inode only upgrades a live `Weak<ExfatFs>` and otherwise remains a read-only snapshot carrier until later cache or data-path owners exist.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add `ExfatInode` as the inode carrier in `inode.rs`.
  - Copy the trusted dentry and chain facts into scalar owner-private fields.
  - Hold `ExfatFs` only through `Weak<ExfatFs>`.
  - Implement the meaningful metadata accessors and `fs()`.
  - Leave read/write, resize, and ownership mutation as explicit temporary seams or rejections.
  - Keep the constructor and any helper surface crate-private unless a later caller is named.
- Explicit non-goals:
  - No inode cache or `InodeKey`.
  - No page-cache backend.
  - No directory ops, namespace mutation, or sync policy.
  - No speculative helper layer for fields that no later caller needs yet.

### Serial Checker Pass

- Required checker-owned tests:
  - Verify the constructor snapshots trusted metadata correctly.
  - Verify `metadata()` agrees with the dedicated metadata accessors.
  - Verify `fs()` upgrades the weak back-reference to the owning filesystem.
  - Verify the temporary seams reject explicitly and do not masquerade as implemented behavior.
- Observable properties that must pass before leaving the serial loop:
  - Metadata identity remains stable after construction.
  - The inode does not require a strong filesystem cycle.
  - The temporary seams are visible and intentional.

### Concurrency Creator Pass

- Required implementation obligations:
  - No dedicated concurrency implementation pass is required for this component.

- Explicit non-goals:
  - Do not add per-inode locking, atomic mutation state, or concurrent writeback scaffolding here.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - N/A.

## Acceptance Notes

- The `EXR-INODE-CORE-17` carrier must read as the stable inode owner, not as a transitional metadata shell.
- The creator pass must not widen into inode-cache or write-side design just because `fs()` and metadata access are easy to implement together.
- If `mod.rs` wiring is needed, it must be treated as a shared-file serialization point with `EXR-FS-CORE-16`.
- The checker should treat any hidden writeback behavior in `set_mode()`, `set_owner()`, or `set_group()` as out of scope for this component.
