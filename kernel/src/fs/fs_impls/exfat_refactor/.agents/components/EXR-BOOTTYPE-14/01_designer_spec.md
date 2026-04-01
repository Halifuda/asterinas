<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Specification

## Metadata

- Component ID: EXR-BOOTTYPE-14
- Title: Validated Boot Sector Typing Boundary
- Status: `Specified`
- Author: main-agent
- Date: 2026-04-01
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOTTYPE-14/00_architect.md`

## Scope

- In scope:
  - Introduce a dedicated validated boot-sector type or equivalent explicit validated wrapper in `boot_sector.rs`.
  - Make the successful validation path return that validated representation instead of only `()`.
  - Make `ExfatSuperBlock` construction consume the validated representation instead of a raw `ExfatBootSector`.
  - Keep the existing boot checksum verification sequencing intact.
  - Add targeted checker-owned ktests that prove the typed boundary is used by the success path and that malformed inputs are still rejected.
- Out of scope:
  - New boot-sector validation rules.
  - Backup Boot Region fallback or comparison policy.
  - Mount object construction, root inode setup, or any filesystem registration changes.
  - FAT, dentry, inode, bitmap, upcase, or page-cache behavior.
  - Broader error-taxonomy refactors outside this narrow boundary.

## Module Specification

- Dependencies:
  - Accepted `EXR-BOOT-01` bootstrap parsing and validation logic.
  - Existing `ExfatSuperBlock` geometry normalization.
  - Existing embedded-image ktest fixture in `test_support.rs`.
- Interfaces provided:
  - `boot_sector.rs` should expose a narrow validated representation, expected in one of these equivalent forms:
    - `ValidatedBootSector(ExfatBootSector)`, or
    - a named validated struct that owns the same validated boot metadata.
  - `validate_primary_boot_sector` should become the constructor for that validated representation, directly or through a small helper.
  - `verify_primary_boot_region_checksum` should consume `&ValidatedBootSector` or another validated borrowable view, not an unchecked raw boot-sector reference.
  - `ExfatSuperBlock` construction should be `From<ValidatedBootSector>` or a similarly explicit conversion from validated state.
  - `read_primary_super_block` should continue to expose `Result<ExfatSuperBlock>` as the top-level bootstrap API.
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- Hidden implementation details:
  - Whether the validated type is a tuple struct, named-field wrapper, or dedicated newtype around `ExfatBootSector`.
  - Whether the validated type exposes `as_ref`, `into_inner`, or a small accessor set, as long as the unchecked raw type is no longer accepted by superblock normalization.

## Functional Specification

- `read_primary_boot_sector` remains the raw disk decode step and still returns `Result<ExfatBootSector>`.
- `validate_primary_boot_sector` must no longer merely certify by convention. On success it must produce an explicit validated representation that later steps can require at the type level.
- `verify_primary_boot_region_checksum` must still run after structural validation and before `ExfatSuperBlock` construction. It may rely on validated geometry fields but must not accept a caller-provided unchecked `ExfatBootSector`.
- `read_primary_super_block` must continue to:
  1. read the raw boot sector,
  2. validate it into the validated representation,
  3. verify the primary boot-region checksum with that validated representation,
  4. normalize into `ExfatSuperBlock`,
  5. return `Result<ExfatSuperBlock>`.
- `ExfatSuperBlock` normalization may keep internal `expect(...)` assertions for already-validated invariants, but those assertions must now be justified by the validated input type instead of an undocumented caller convention.

## Invariants

- Raw `ExfatBootSector` still represents only decoded on-disk bytes, not trusted geometry.
- The validated wrapper represents a boot sector that has passed all current structural checks in `validate_primary_boot_sector`.
- No code path should be able to construct an `ExfatSuperBlock` from an unchecked `ExfatBootSector`.
- Checksum verification still depends on already-validated sector geometry and still runs before the superblock is returned to callers.
- Existing malformed-input rejection behavior remains intact.

## Concurrency Specification

- Shared state:
  - borrowed `BlockDevice` only
- Lock ordering:
  - none
- Atomicity requirements:
  - unchanged from the accepted boot component
- Forbidden interleavings:
  - do not add caches, global mutable validation state, or mount-wide synchronization
- Allowed simplifications such as a temporary big lock:
  - no new concurrency machinery is required for this cleanup component

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - add the validated boot-sector representation,
  - thread it through validation, checksum verification, and superblock normalization,
  - update call sites and tests to use the new type boundary,
  - keep visibility narrow and keep the API focused on the bootstrap path only.
- Explicit non-goals:
  - do not add new validation rules,
  - do not redesign the whole boot parser,
  - do not widen scope into mount or later filesystem objects.

### Serial Checker Pass

- Required checker-owned tests:
  - rerun the existing boot success and malformed-input ktests affected by the boundary change,
  - add one targeted ktest or compile-visible assertion path that would fail if unchecked boot sectors could still flow directly into superblock normalization.
- Observable properties that must pass before leaving the serial loop:
  - the success path still builds an `ExfatSuperBlock`,
  - malformed boot sectors are still rejected,
  - the type boundary is no longer implicit in comments or call order alone.

### Concurrency Creator Pass

- Required implementation obligations:
  - no dedicated concurrency implementation required
- Explicit non-goals:
  - do not invent synchronization work for this component

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - the component remains read-only and free of helper-owned shared mutable state.

## Acceptance Notes

- Reviewer should pay close attention to whether the validated type actually strengthens the API boundary or merely renames the old convention.
- Reviewer should also check for needless accessor sprawl or wrapper churn that makes later bootstrap code harder to read.
