<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Proposal: Owner-First Board Reset

## Metadata

- Component ID: `WORKSPACE-ARCH-RESET`
- Title: Scheduler-Owned Board Redesign For `exfat_refactor` Under Owner-First Rules
- Status: `Architected` (proposal artifact; does not itself enter implementation)
- Author: architect (delegated subagent)
- Date: 2026-04-05
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/WORKSPACE-ARCH-RESET/20260405-1800-architect-packet.md`

## Problem Statement (What This Reset Fixes)

The current board (reset to the post-`EXR-SBGEOM-15` baseline) correctly established low-level validated-value layers, but the failed wave starting at `EXR-INOKEY-05A` drifted into ownerless staging surfaces that did not converge on the stable Asterinas integration owners: the VFS `FileSystem` trait carrier and the VFS `Inode` trait carrier.

This proposal replaces the board with an owner-first unit graph where every tracked unit:

- serves a concrete filesystem function in the finished system, and
- lands in a stable final architectural owner (trait carrier, runtime owner, or validated value type),
- without inventing standalone “packet convenience” modules solely to enable parallelism.

## Prior Sources That Materially Shaped This Split

- Rollback rationale and “what failed”: `.agents/main-agent/stone-lantern-20260405-1721-rollback-baseline.md`.
- Current board and accepted component inventory: `.agents/COMPONENT_INDEX.md`.
- Owner-first scheduler terminology and gates: `.agents/PROTOCOL.md`, plus role rules in `.agents/protocol/COMMON_SUBAGENT.md` and `.agents/protocol/ARCHITECT.md`.
- Asterinas integration constraints (what must ultimately exist): `.agents/ASTERINAS_ARCHITECT_PRIORS.md`.
- VFS trait carriers and method surfaces that “real owners” must implement:
  - `kernel/src/fs/vfs/fs_apis/file_system.rs`
  - `kernel/src/fs/vfs/fs_apis/inode.rs`
  - `kernel/src/fs/vfs/fs_apis/inode_ext.rs`
- Current `exfat_refactor` code layout for already-landed foundations:
  - `kernel/src/fs/fs_impls/exfat_refactor/{boot_sector,super_block,io,fat,dentry,fileset}.rs`
- Linux implementation map for coherence checks (not as semantic authority): `.agents/linux-exFAT-implementation-summary.md`.

## Owner-First “Finished System” Ownership Tree (Stable Owners)

This refactor should converge on these stable owners, in this order:

1. **Trait carriers (public integration owners)**
   - `ExfatFs`: implements VFS `FileSystem` for an exFAT mount instance.
   - `ExfatInode`: implements VFS `Inode` for exFAT inodes (regular file and directory at minimum).
2. **Runtime state owners (internal but stable)**
   - `OpenedInodeTable`: `ExfatFs`-owned cache keyed by a stable `InodeKey`.
   - `UpcaseTable`: `ExfatFs`-owned validated table and name-normalization services.
   - `AllocationBitmap`: `ExfatFs`-owned bitmap state; read-only queries first, later mutation.
   - `FatTable`: `ExfatFs`-owned FAT access and (later) mutation policy.
   - `DirectoryEngine`: `ExfatFs`-owned directory reader/writer that operates on chains and file-record sets.
3. **Validated value types (stable “leaf owners”)**
   - `ValidatedBootSector`, normalized `ExfatSuperBlock` geometry.
   - `ExfatChain` (read-only traversal now; later may gain write-side helpers but stays a value+invariant boundary).
   - `FatValue` decoding model.
   - `RawExfatDentry` and typed `ExfatDentry`.
   - `ExfatDentrySet` as the validated file-record invariant boundary.
   - `InodeKey` as a validated value type (derived from a dentry-set location in a directory).

Key constraint: “mount/open sequencing” is not its own long-lived owner. It must land as `ExfatFs::open(...)` (or equivalent constructor) and use internal services above.

## Proposed Replacement Unit Graph (Tracked Functional Units)

The graph is expressed as *functional units*, not work slices. Each unit may still be implemented via multiple creator slices, but its boundary must remain stable and owner-justified.

Legend:

- **Final owner**: the stable owner in the finished system.
- **Landing form**: owner methods / owner-private helpers / owner-internal state / validated value type.
- **Boundary kind**:
  - stable architectural boundary (tracked functional unit),
  - independent validated value type (tracked functional unit),
  - or temporary construction seam (tracked only when it has an explicit exit plan).

### Foundation Layer (Keep As Tracked Validated-Value Units)

These are architecturally real as independent validated value types and are already accepted. Keep them tracked as “foundation” rather than treating them as staging.

| Proposed Unit ID | Functional Goal Served | Final Owner | Landing Form | Boundary Kind | Depends On |
| --- | --- | --- | --- | --- | --- |
| `EXR-BOOT-01` | Establish trusted volume boot facts and normalized geometry | `ValidatedBootSector` + `ExfatSuperBlock` | validated value types | independent validated value type | None |
| `EXR-IO-02` | Provide aligned metadata byte reads for exFAT metadata | `ExfatFs` (internal I/O helper) | owner-private helper | stable architectural boundary (foundation) | `EXR-BOOT-01` |
| `EXR-FATVAL-03A` | Decode one FAT entry into a typed value model | `FatValue` | validated value type | independent validated value type | `EXR-BOOT-01`, `EXR-IO-02` |
| `EXR-CHAIN-03B` | Read-only cluster-chain state and traversal | `ExfatChain` | validated value type | independent validated value type | `EXR-FATVAL-03A` |
| `EXR-DENTRY-04A` | Raw dentry layout and typed one-entry decode | `RawExfatDentry`/`ExfatDentry` | validated value types | independent validated value type | `EXR-BOOT-01` |
| `EXR-FILESET-04B` | Validated multi-entry file-record set + checksum boundary | `ExfatDentrySet` | validated value type | independent validated value type | `EXR-DENTRY-04A` |

Notes:

- `EXR-BOOTTYPE-14` and `EXR-SBGEOM-15` are accepted cleanup slices that tighten `EXR-BOOT-01`’s output typing and geometry semantics. In the replacement board they should either remain as historical accepted rows or be explicitly “folded into boot foundations” as non-repeating maintenance units. They should not be treated as templates for future ownerless staging cuts.

### Convergence Layer 1 (Introduce The Real Trait Carriers Early)

These are the first new tracked units. They exist solely to force convergence on stable owners (`ExfatFs`, `ExfatInode`) and to prevent the “helper-first drift” that triggered the rollback.

| Proposed Unit ID | Functional Goal Served | Final Owner | Landing Form | Boundary Kind | Depends On |
| --- | --- | --- | --- | --- | --- |
| `EXR-FS-CORE-16` | Define `ExfatFs` runtime state and implement VFS `FileSystem` skeleton (read-only) | `ExfatFs` | trait-carrier type + owner state | stable architectural boundary | `EXR-BOOT-01` |
| `EXR-INODE-CORE-17` | Define `ExfatInode` and implement required VFS `Inode` core methods (read-only) | `ExfatInode` | trait-carrier type + owner state | stable architectural boundary | `EXR-FS-CORE-16`, `EXR-FILESET-04B`, `EXR-CHAIN-03B` |
| `EXR-INODE-CACHE-18` | Opened-inode table keyed by stable `InodeKey` (root special case included) | `ExfatFs` | owner-internal state + validated `InodeKey` | stable architectural boundary | `EXR-FS-CORE-16`, `EXR-INODE-CORE-17` |

Boundary justification:

- `EXR-INODE-CORE-17` and `EXR-INODE-CACHE-18` are *not* “packet convenience.” The inode type and inode-identity cache are the stable Asterinas integration boundary that later lookup/read/write/rename must live under.
- `InodeKey` is permitted as an independent validated value type, but it must be introduced explicitly as “the cache key for `OpenedInodeTable` and a stable identity for `ExfatInode`,” not as a freestanding helper module.

### Convergence Layer 2 (Mount/Open As `ExfatFs` Behavior, Not A Separate Owner)

Mount/open is real behavior, but not a separate architectural owner. It must land as `ExfatFs::open(...)` or equivalent constructor and should be tracked as part of a filesystem-owner unit, with internal services split only when they have stable owners.

| Proposed Unit ID | Functional Goal Served | Final Owner | Landing Form | Boundary Kind | Depends On |
| --- | --- | --- | --- | --- | --- |
| `EXR-DIR-ENGINE-19` | Read directory contents as `ExfatDentrySet` streams over a directory chain | `ExfatFs` | owner-internal service (`DirectoryEngine`) | stable architectural boundary | `EXR-IO-02`, `EXR-CHAIN-03B`, `EXR-FILESET-04B` |
| `EXR-UPCASE-20` | Load/validate upcase table and provide name-folding + hash services | `ExfatFs` | owner-internal state (`UpcaseTable`) + methods | stable architectural boundary | `EXR-DIR-ENGINE-19` |
| `EXR-BITMAP-21` | Load/validate allocation bitmap and provide occupancy queries | `ExfatFs` | owner-internal state (`AllocationBitmap`) + methods | stable architectural boundary | `EXR-DIR-ENGINE-19` |
| `EXR-FS-OPEN-22` | Implement `ExfatFs::open(...)`: boot -> root inode -> sys entries -> ready root | `ExfatFs` | owner methods + sequencing invariants | stable architectural boundary | `EXR-FS-CORE-16`, `EXR-INODE-CORE-17`, `EXR-INODE-CACHE-18`, `EXR-UPCASE-20`, `EXR-BITMAP-21` |

Boundary justification:

- `EXR-DIR-ENGINE-19` is architecturally real because it is the shared “directory-as-record-stream” engine used by both mount-time sys-entry discovery and `ExfatInode` directory operations. It is a stable internal service under `ExfatFs` (not a free-function staging module).
- `EXR-UPCASE-20` and `EXR-BITMAP-21` are stable long-lived pieces of filesystem runtime state; they are not “mount-only.” They deserve explicit ownership under `ExfatFs` with documented invariants and (later) lock order.

### Read-Only VFS Behavior Layer (Make Direct Progress Toward User-Visible Ops)

After `EXR-FS-OPEN-22`, the board should prioritize user-visible read-only behavior on the `Inode` trait carrier, using the stable internal services above.

| Proposed Unit ID | Functional Goal Served | Final Owner | Landing Form | Boundary Kind | Depends On |
| --- | --- | --- | --- | --- | --- |
| `EXR-DIR-OPS-23` | Implement `lookup` and `readdir_at` for directory `ExfatInode` | `ExfatInode` | owner methods calling `DirectoryEngine` + `UpcaseTable` | stable architectural boundary | `EXR-FS-OPEN-22`, `EXR-DIR-ENGINE-19`, `EXR-UPCASE-20` |
| `EXR-FILE-MAP-24` | Logical-offset to physical mapping for regular files (read-only) | `ExfatInode` | owner-private helpers | stable architectural boundary | `EXR-CHAIN-03B`, `EXR-INODE-CORE-17` |
| `EXR-READ-OPS-25` | Implement buffered `read_at` semantics (incl. zero-fill rules) | `ExfatInode` | owner methods | stable architectural boundary | `EXR-FILE-MAP-24` |
| `EXR-PGCACHE-26` | Implement `PageCacheBackend` integration for exFAT inodes | `ExfatInode` | owner-internal state + trait impl | stable architectural boundary | `EXR-READ-OPS-25` |

Note: the board should treat “read-only first” as a deliberate risk-control strategy. It produces VFS-visible progress without forcing early correctness in rename/truncate/sync ordering.

### Write-Side Layer (State Mutation Under The Correct Owners)

Write-side work is where ownerless helper drift is most likely. The board should only track write-side units that land in stable owners and explicitly state concurrency/atomicity boundaries during design.

| Proposed Unit ID | Functional Goal Served | Final Owner | Landing Form | Boundary Kind | Depends On |
| --- | --- | --- | --- | --- | --- |
| `EXR-ALLOC-27` | Allocation search/mark/free with dirty tracking; couples bitmap + FAT mutation | `ExfatFs` | owner-internal service (`Allocator`) | stable architectural boundary | `EXR-BITMAP-21`, `EXR-FATVAL-03A`, `EXR-IO-02` |
| `EXR-DENTRY-WRITE-28` | Update directory file-record sets on disk (create/delete/rename primitives) | `ExfatFs` + `ExfatInode` | `DirectoryEngine` write methods | stable architectural boundary | `EXR-DIR-ENGINE-19`, `EXR-FILESET-04B`, `EXR-ALLOC-27` |
| `EXR-NAMESPACE-29` | Implement `create/unlink/mkdir/rmdir/rename` on `ExfatInode` | `ExfatInode` | owner methods | stable architectural boundary | `EXR-DIR-OPS-23`, `EXR-DENTRY-WRITE-28`, `EXR-UPCASE-20` |
| `EXR-WRITE-30` | Implement buffered `write_at`, growth, and `resize` | `ExfatInode` | owner methods | stable architectural boundary | `EXR-PGCACHE-26`, `EXR-ALLOC-27` |
| `EXR-SYNC-31` | Implement `sync`, `sync_data`, and filesystem-wide `sync()` ordering | `ExfatFs` | owner methods | stable architectural boundary | `EXR-NAMESPACE-29`, `EXR-WRITE-30` |

## Proposed Parallel Waves (Command-Free Lanes Only)

This is not a creator plan; it is the minimal wave guidance needed to preserve parallelism without inventing fake boundaries.

1. Wave A (trait-carrier convergence): `EXR-FS-CORE-16` and `EXR-INODE-CORE-17` can be specified in parallel once architected, but implementation should be sequenced so `ExfatFs` owns the shared state first.
2. Wave B (mount-critical internal services): `EXR-DIR-ENGINE-19`, `EXR-UPCASE-20`, `EXR-BITMAP-21` can be designed in parallel; `EXR-FS-OPEN-22` depends on all three.
3. Wave C (read-only user-visible ops): `EXR-DIR-OPS-23` can run in parallel with `EXR-FILE-MAP-24` once `EXR-FS-OPEN-22` is stable.
4. Wave D (write-side): `EXR-ALLOC-27` and `EXR-DENTRY-WRITE-28` must be architected and designed with explicit lock order and atomicity rules before `EXR-NAMESPACE-29` begins.

## Mapping From Current Board IDs To This Proposal

This mapping is intended to let the main agent rebuild `COMPONENT_INDEX.md` without losing already-accepted work while still correcting the post-`INOKEY` architectural drift.

### Keep (as tracked foundation units)

- `EXR-BOOT-01` stays as-is (foundation validated-value boundary).
- `EXR-IO-02` stays as-is (foundation helper; functionally used by `ExfatFs` internals).
- `EXR-FATVAL-03A` stays as-is.
- `EXR-CHAIN-03B` stays as-is.
- `EXR-DENTRY-04A` stays as-is.
- `EXR-FILESET-04B` stays as-is.

### Reclassify / Fold (accepted maintenance slices)

- `EXR-BOOTTYPE-14`: fold into “boot foundations” as a tightening slice; do not treat as a pattern for future splitting unless the unit is also a validated-value boundary.
- `EXR-SBGEOM-15`: fold into “boot foundations”; it represents `ExfatSuperBlock` geometry invariants that the filesystem owner (`ExfatFs`) will consume.

### Replace (planned units that were packet-convenience cuts)

- `EXR-INOKEY-05A` -> replace with `EXR-INODE-CACHE-18` (introduce `InodeKey` only as a cache key/value type owned by `ExfatFs`).
- `EXR-INODE-05B` -> replace with `EXR-INODE-CORE-17` (trait carrier first; metadata shell is a work-slice, not a unit boundary).
- `EXR-SYSROOT-06` -> replace with `EXR-FS-OPEN-22` + `EXR-DIR-ENGINE-19` (sys-entry discovery is mount sequencing behavior owned by `ExfatFs`).
- `EXR-UPCASE-07A`/`EXR-UPCASE-07B` -> replace with `EXR-UPCASE-20` (keep as one stable `UpcaseTable` owner; any sub-splitting should be work slices, not tracked units).
- `EXR-BITMAP-08A`/`EXR-BITMAP-08B` -> replace with `EXR-BITMAP-21` + later `EXR-ALLOC-27` (read-only bitmap state is a stable `ExfatFs` field; mutation policy belongs to allocator).
- `EXR-MOUNT-09` -> replace with `EXR-FS-CORE-16` + `EXR-FS-OPEN-22` (mount is `ExfatFs` behavior, not a standalone owner).
- `EXR-DIR-10` -> replace with `EXR-DIR-OPS-23` + `EXR-DIR-ENGINE-19` (VFS-visible dir ops live on `ExfatInode`; record streaming lives in an `ExfatFs` internal service).
- `EXR-READ-11A`/`EXR-READ-11B` -> replace with `EXR-FILE-MAP-24` + `EXR-READ-OPS-25` (both owned by `ExfatInode`).
- `EXR-PGCACHE-11B` -> replace with `EXR-PGCACHE-26` (explicitly owned by `ExfatInode` and dependent on the read path).
- `EXR-CREATE-12A`/`EXR-CREATE-12B`/`EXR-RENAME-13D` -> replace with `EXR-NAMESPACE-29` (single owner: `ExfatInode` dir behavior; internal primitives in `DirectoryEngine`).
- `EXR-WRITE-13A`/`EXR-WRITE-13B`/`EXR-WRITE-13C` -> replace with `EXR-ALLOC-27` + `EXR-WRITE-30` (allocator is `ExfatFs` internal; write/truncate is inode behavior).
- `EXR-SYNC-13E` -> replace with `EXR-SYNC-31` (filesystem-wide ordering owned by `ExfatFs`).

## Boundary Rejections (Explicit)

These are the key “reject even if it helps scheduling” decisions needed to avoid repeating the rollback drift:

- Reject “inode identity key” as a standalone tracked component not tied to a stable runtime owner. `InodeKey` may exist, but only as a validated value type used by `OpenedInodeTable` and stored in `ExfatInode`.
- Reject “system root scanner” as a standalone unit. It is mount sequencing owned by `ExfatFs` and should not become a long-lived abstraction boundary.
- Reject splitting “directory iteration” into multiple free helper modules. The stable boundaries are:
  - `DirectoryEngine` (an `ExfatFs` internal service that streams/updates `ExfatDentrySet`),
  - and VFS-visible `ExfatInode` directory methods that consume that service.
- Reject “mount object” as something other than `ExfatFs`. Asterinas integration requires the `FileSystem` trait carrier; do not interpose a staging owner between VFS and `ExfatFs`.

## Migration Recommendation For Rebuilding `COMPONENT_INDEX.md` (Main-Agent Action)

This proposal is intentionally “replacement-ready” but does not edit the board.

Recommended main-agent rebuild procedure:

1. Keep the already accepted foundation components as accepted rows (no artifact migration required).
2. Add new planned rows for `EXR-FS-CORE-16` through `EXR-SYNC-31` in dependency order.
3. Mark the old planned rows (`EXR-INOKEY-05A` through `EXR-SYNC-13E`) as retired/replaced in notes (or remove them if the board format prefers a clean slate), using the mapping above so older packets are not misinterpreted as current plan.
4. Enforce the owner-first policy at the board level: every new row’s “Notes” should explicitly name its final architectural owner and landing form (`ExfatFs` method/service/field vs `ExfatInode` method/field vs validated value type).

## Risks And Open Questions (Irreducible At Architect Stage)

- Concurrency and lock ordering for `ExfatFs` shared state (bitmap/FAT/inode table) must be made explicit during design of `EXR-ALLOC-27` and `EXR-DENTRY-WRITE-28`. The architect stage can only force ownership; it cannot safely “guess” the lock order.
- Whether upcase/name behavior should be UTF-16-only initially or also accept UTF-8 inputs is a semantic policy choice that must be recorded in designer specs; this proposal only forces ownership (`UpcaseTable` under `ExfatFs`).
- Writeback ordering (directory entry updates vs FAT/bitmap updates vs page cache) must be treated as an explicit design obligation for `EXR-NAMESPACE-29`, `EXR-WRITE-30`, and `EXR-SYNC-31`.
