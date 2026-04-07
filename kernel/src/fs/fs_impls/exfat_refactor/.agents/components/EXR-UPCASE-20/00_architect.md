<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-UPCASE-20`
- Title: `ExfatFs` upcase-table owner boundary
- Status: `Architected`
- Author: architect
- Date: `2026-04-07`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-20/20260407-1110-architect-packet.md`

## Functional Unit Definition

- Functional goal: load and validate the exFAT upcase table, then provide stable case-folding and exFAT name-hash services for later lookup and namespace work.
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-internal state (`UpcaseTable`) plus owner methods
- Boundary kind: stable architectural boundary
- Why this boundary is architecturally real: case folding and name hashing are filesystem-wide canonicalization services. They depend on mount-time volume metadata, they must remain stable for the lifetime of the mounted filesystem, and they are consumed by later directory and namespace owners. They are not directory scanning, not VFS directory ops, and not a free helper namespace.

## Purpose

This unit is the smallest coherent slice that gives `ExfatFs` ownership of upcase-derived behavior without widening into directory traversal or namespace mutation.
`DirectoryEngine` remains the source of raw singleton `Upcase` candidates.
`EXR-UPCASE-20` is responsible for validating the candidate, materializing the table, and exposing folding/hash services from the filesystem owner.

The owner boundary should therefore answer three questions only:

1. how the validated table is stored under `ExfatFs`,
2. how UTF-16 code units are folded through that table,
3. how exFAT name hashes are computed from folded name units.

Everything else stays elsewhere.

## Why This Comes Now

The board already has the owner split needed to keep this unit stable:

- `EXR-DIR-ENGINE-19` owns the read-only record stream and surfaces raw `Upcase` candidates.
- `EXR-FS-CORE-16` already establishes `ExfatFs` as the filesystem-wide owner.
- `EXR-INODE-CORE-17` and `EXR-INODE-CACHE-18` keep inode identity and opened-handle reuse out of this boundary.

That means the upcase service can be defined now as an `ExfatFs` concern, even though `EXR-FS-OPEN-22` will later wire the mount sequence that actually installs the table into a live filesystem instance.

## Owner And Integration Convergence

- Interfaces, traits, services, or higher-level functions this unit ultimately serves: future `EXR-FS-OPEN-22`, future `EXR-DIR-OPS-23`, future `EXR-NAMESPACE-29`, and any later name-comparison or hash-consumer paths.
- If the unit is internal-only, why that internal ownership is still stable in the finished system: it is not a staging helper. `UpcaseTable` is a filesystem-wide runtime service under `ExfatFs`, and later directory or namespace owners must consume it rather than reimplementing canonicalization.
- Known non-goals or nearby logic that must remain in the parent owner: raw directory scanning, mount/open sequencing, inode cache policy, bitmap policy, VFS `lookup`/`readdir_at`, and namespace mutation all stay outside this unit.

Boundary consumption rules:

- `DirectoryEngine` may surface a raw `Upcase` candidate, but it must not load the table or case-fold names.
- `UpcaseTable` should validate the candidate and own the decoded table bytes, but it must not become a directory scanner.
- Name comparison and hashing must remain owner methods on `ExfatFs` or its owner-private helpers, not a free text utility module.

## Dependency Contract

- Depends on: `EXR-DIR-ENGINE-19`, the validated boot and superblock facts already accepted for the refactor, and the exFAT name-encoding facts already present in `EXR-FILESET-04B` / `EXR-DENTRY-04A`.
- Blocks: later directory lookup and rename/name-resolution work in `EXR-DIR-OPS-23` and `EXR-NAMESPACE-29`, plus mount sequencing in `EXR-FS-OPEN-22`.
- Can run in parallel with: `EXR-BITMAP-21` architect/design work and later creator work, because both are filesystem-owned consumers of `DirectoryEngine` rather than owners of directory scanning themselves.
- Recommended parallel wave: Wave B after `EXR-DIR-ENGINE-19` is specified; keep the upcase owner in `ExfatFs`, but do not force this unit to absorb raw directory discovery.
- Stable pre-existing interfaces used: `ExfatFs`, `ExfatDentrySet`, `ExfatDentry`, `DirectoryEngine` candidate output, and the validated boot/superblock geometry already accepted for the refactor.
- Prior sources or prior slices that materially shaped the split: `WORKSPACE-ARCH-RESET/00_architect.md`, `EXR-DIR-ENGINE-19/00_architect.md`, `EXR-FS-CORE-16/00_architect.md`, `EXR-INODE-CORE-17/00_architect.md`, `EXR-INODE-CACHE-18/00_architect.md`, `ASTERINAS_ARCHITECT_PRIORS.md` profile `I-ARCH`, `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-ARCH`, and `linux-exFAT-implementation-summary.md` topic "Upcase table and charset behavior".

## Recommended Work Slices

These are candidate creator slices for scheduler consideration, not the active global plan.

| Slice ID | Parent Unit Scope | Goal | Likely Write Set | Depends On | May Overlap With | Lane Class | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-UPCASE-20-A` | `EXR-UPCASE-20` | Define `UpcaseTable` as `ExfatFs`-owned runtime state, accept the raw singleton `Upcase` candidate from mount flow, and validate table size and checksum before the table becomes live. | `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` | `EXR-DIR-ENGINE-19`, `EXR-FS-CORE-16` | `EXR-BITMAP-21` architect only at the scheduler level; not file-parallel with any sibling lane that still needs `fs.rs` wiring | creator | Keep this slice focused on ownership and validation. Do not scan directories here and do not pull name-comparison policy into mount sequencing. |
| `WS-UPCASE-20-B` | `EXR-UPCASE-20` | Add the owner-private folding and name-hash services that read the validated table and serve later lookup and namespace callers. | `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` | `WS-UPCASE-20-A` | `EXR-DIR-OPS-23` architect/design only; not file-parallel with `WS-UPCASE-20-A` if both remain in `fs.rs` | creator | This slice is still owner-local service code. It should not become a generic text helper or a directory engine adjunct. |

## exFAT Concepts Covered

- Upcase-table discovery from the root-directory metadata stream.
- On-disk upcase-table size and checksum validation.
- UTF-16 case folding through the validated volume table.
- exFAT name-hash computation from folded name units.
- Filesystem-wide canonicalization state under `ExfatFs`.
- Separation of raw singleton candidate discovery from table ownership.

## Boundary Rejections

- Splitting upcase handling into a free helper module was rejected. That would be packet convenience, not a stable owner boundary.
- Folding directory scanning into this unit was rejected. `DirectoryEngine` already owns the raw candidate stream.
- Folding VFS directory lookup or rename/name-resolution policy into this unit was rejected. Those belong to later inode-facing owners.
- Treating the upcase table as a mount-only throwaway object was rejected. It is stable filesystem runtime state under `ExfatFs`.
- Turning `UpcaseTable` into a directory or namespace service was rejected. Its job is canonicalization, not traversal or mutation.

## Target Files

- Existing files likely to change: `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- New files expected: none required by this boundary; if a future extraction becomes necessary, it must preserve `ExfatFs` as the owner and not create a separate helper owner
- Future collision risk to watch: `mod.rs` only if the implementation later needs a new private module, and `fs.rs` if the upcase owner shares the same file with `EXR-FS-CORE-16` or `EXR-INODE-CACHE-18`

## Code Budget

- Target creator work-slice size: `180-280` lines
- Expected number of creator slices: `2`
- Reason if any single slice might exceed 500 lines: it should not. If it does, the slice has probably absorbed directory scanning or namespace behavior, which means the boundary has drifted.

## Exit Condition

Design work may start once `ExfatFs` clearly owns a validated upcase table and can answer name-folding and name-hash requests from that table, while `DirectoryEngine` remains the only source of raw `Upcase` candidates.

## Risks

- The candidate load path can accidentally become a hidden directory scanner if it starts searching for the upcase entry itself.
- The folding service can accidentally drift into a general text API if later callers are not named and constrained.
- The mount-time installation point belongs to `EXR-FS-OPEN-22`; if this component starts owning sequencing, it has widened too far.
- Whether the filesystem should keep a default fallback uppercase table in addition to the validated volume table is a semantic choice that designer work must pin down; the ownership boundary does not change either way.
