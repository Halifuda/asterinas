<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Specification

## Metadata

- Component ID: EXR-BOOT-01
- Title: Boot Region Parsing And Normalized Runtime Geometry
- Status: `Specified`
- Author: designer
- Date: 2026-03-31
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-01/00_architect.md`

## Scope

- In scope:
  - Read sector `0` of the volume into an `ExfatBootSector` value.
  - Validate the primary boot sector and the primary boot region checksum.
  - Reject malformed boot metadata before any filesystem object is built.
  - Convert validated boot metadata into a normalized `ExfatSuperBlock`.
  - Preserve legacy-comparable runtime type names `ExfatBootSector` and `ExfatSuperBlock`.
  - Define targeted checker-owned `#[ktest]` obligations for the success path and malformed-boot failures.
- Out of scope:
  - Backup Boot Region fallback, comparison, or recovery policy.
  - Construction of `ExfatFs`, root inode, upcase table, bitmap, or FAT chain objects.
  - Volume-dirty mutation, clear-to-zero behavior, or any writeback to the boot region.
  - Mount-option parsing, NLS or UTF-8 setup, warning policy, and filesystem registration.
  - Directory, FAT-entry, inode, page-cache, or namespace behavior beyond geometry facts needed by later components.

## Module Specification

- Dependencies:
  - Stable kernel block-device reads already used by the legacy exFAT code.
  - exFAT constants for boot signature, reserved clusters, sector-size bounds, and persistent volume flags.
  - Targeted kernel tests using the existing in-memory block-device pattern from legacy exFAT tests.
- Interfaces provided:
  - `ExfatBootSector` as the packed on-disk boot-sector representation for sector `0`.
  - `ExfatSuperBlock` as the normalized runtime geometry and boot-state structure for later `exfat_refactor` components.
  - A read-only high-level loader that reads the primary boot sector, validates primary-boot metadata plus checksum, and returns `ExfatSuperBlock`.
  - Narrow helper functions for:
    - boot-sector field validation,
    - primary-boot checksum verification,
    - runtime-geometry normalization.
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- Hidden implementation details:
  - How raw boot-region bytes are buffered before checksum verification.
  - Whether the top-level loader is implemented as one public helper plus private validation helpers, or as small public functions re-exported from `mod.rs`.
  - Any internal helper type used to hold validated intermediate facts before creating `ExfatSuperBlock`.

The creator must keep module boundaries narrow:

- `boot_sector.rs` owns on-disk layout, read-time validation helpers, and primary-boot checksum verification.
- `super_block.rs` owns `ExfatSuperBlock` and normalization from a validated boot sector.
- `mod.rs` only wires modules and test support. It must not become a catch-all implementation file.

## Functional Specification

### Operation

- Name: `read_primary_boot_sector`
- Inputs:
  - `block_device: &dyn BlockDevice`
- Preconditions:
  - The caller is in the mount bootstrap path before any `ExfatFs` object is created.
  - `block_device` can service a read of sector `0`.
- Actions:
  - Read sector `0` as `ExfatBootSector`.
  - Treat the on-disk structure as little-endian exFAT boot metadata.
  - Do not read any other sector and do not perform validation beyond what is required to deserialize sector `0`.
- Outputs:
  - `Result<ExfatBootSector>`
- Postconditions:
  - On success, the returned value is an exact decode of sector `0`.
  - No filesystem-global state is created or mutated.
- Error cases:
  - Propagate device or decoding failures as I/O-style errors.

### Operation

- Name: `validate_primary_boot_sector`
- Inputs:
  - `boot_sector: &ExfatBootSector`
- Preconditions:
  - `boot_sector` came from sector `0` of the target volume.
- Actions:
  - Validate boot metadata using exFAT rules and the current main-agent scope decision.
  - The component must reject at least these conditions:
    - `signature != 0xAA55`
    - `fs_name != b"EXFAT   "`
    - any byte in `must_be_zero` is nonzero
    - `num_fats` is neither `1` nor `2`
    - `sector_size_bits` is outside `9..=12`
    - `sector_size_bits + sector_per_cluster_bits > 25`
    - `fat_offset < 24`
    - `fat_length == 0`
    - `cluster_count == 0`
    - `root_cluster < EXFAT_RESERVED_CLUSTERS`
    - `root_cluster > cluster_count + EXFAT_RESERVED_CLUSTERS - 1`
    - the FAT area is too small to store one `u32` entry per cluster in the normalized cluster-number space
    - `cluster_offset < fat_offset + fat_length * num_fats`
    - `vol_length <= cluster_offset`
  - This operation must not decide backup-boot fallback policy and must not emit volume-state warnings.
- Outputs:
  - `Result<()>`
- Postconditions:
  - Success means later normalization may trust the validated boot-sector fields without repeating structural checks.
  - Failure means no normalized runtime geometry is produced.
- Error cases:
  - Malformed or inconsistent boot metadata returns invalid-data style errors.

### Operation

- Name: `verify_primary_boot_region_checksum`
- Inputs:
  - `block_device: &dyn BlockDevice`
  - `boot_sector: &ExfatBootSector`
- Preconditions:
  - `validate_primary_boot_sector(boot_sector)` has succeeded.
  - `sector_size_bits` from `boot_sector` can be used to derive bytes per sector.
- Actions:
  - Read the first `11` sectors of the Main Boot Region and the checksum sector at index `11`.
  - Compute the exFAT boot checksum across the first `11 * bytes_per_sector` bytes by rotating the `u32` accumulator right by one bit and adding each byte value.
  - Skip bytes `106`, `107`, and `112` of the boot region during checksum calculation, as required by the exFAT specification.
  - Interpret sector `11` as an array of little-endian `u32` checksum entries.
  - Require every checksum entry in sector `11` to equal the computed checksum.
  - Do not read, compare, or repair the Backup Boot Region.
- Outputs:
  - `Result<()>`
- Postconditions:
  - Success means the primary Main Boot Region checksum matches the on-disk checksum sector.
  - No persistent state or caches are mutated.
- Error cases:
  - Any device-read failure propagates as an I/O-style error.
  - Any mismatch between the computed checksum and any checksum entry returns an invalid-data style error.

### Operation

- Name: `normalize_super_block`
- Inputs:
  - `boot_sector: ExfatBootSector`
- Preconditions:
  - `validate_primary_boot_sector(&boot_sector)` has succeeded.
- Actions:
  - Construct `ExfatSuperBlock` with these exact normalized rules:
    - `sector_size = 1 << sector_size_bits`
    - `sect_per_cluster = 1 << sector_per_cluster_bits`
    - `cluster_size_bits = sector_size_bits + sector_per_cluster_bits`
    - `cluster_size = 1 << cluster_size_bits`
    - `fat1_start_sector = fat_offset as u64`
    - `fat2_start_sector = fat1_start_sector` when `num_fats == 1`, otherwise `fat1_start_sector + fat_length as u64`
    - `num_fat_sectors = fat_length`
    - `data_start_sector = cluster_offset as u64`
    - `num_sectors = vol_length`
    - `num_clusters = cluster_count + EXFAT_RESERVED_CLUSTERS`
    - `root_dir = root_cluster`
    - `vol_flags = vol_flags as u32`
    - `vol_flags_persistent = vol_flags & (VOLUME_DIRTY | MEDIA_FAILURE)`
    - `cluster_search_ptr = EXFAT_FIRST_CLUSTER`
    - `used_clusters = !0`
    - `dentries_per_clu = cluster_size / 32`
  - Preserve the legacy interpretation that `num_clusters` is the maximum valid cluster identifier rather than the raw on-disk `ClusterCount`.
- Outputs:
  - `ExfatSuperBlock`
- Postconditions:
  - The result contains only normalized geometry and boot-state facts needed by later components.
  - The result does not own block-device handles, caches, or mutable mount policy.
- Error cases:
  - None beyond earlier validation; this operation is infallible once its preconditions hold.

### Operation

- Name: `read_primary_super_block`
- Inputs:
  - `block_device: &dyn BlockDevice`
- Preconditions:
  - The caller is performing the read-only bootstrap path for `exfat_refactor`.
- Actions:
  - Call `read_primary_boot_sector`.
  - Call `validate_primary_boot_sector`.
  - Call `verify_primary_boot_region_checksum`.
  - Call `normalize_super_block`.
  - Return the resulting `ExfatSuperBlock`.
- Outputs:
  - `Result<ExfatSuperBlock>`
- Postconditions:
  - On success, the caller receives normalized geometry for later bootstrapping work.
  - No `ExfatFs`, inode, bitmap, FAT cache, or upcase-table state exists yet.
  - No warning logs are emitted from this component.
- Error cases:
  - Propagate the first failing I/O or invalid-data error from the helper operations.

## Invariants

- `sector_size` is always in the inclusive range `512..=4096`.
- `sect_per_cluster` is always a power of two.
- `cluster_size == sector_size * sect_per_cluster`.
- `cluster_size_bits == sector_size_bits + sector_per_cluster_bits`.
- `fat1_start_sector < data_start_sector`.
- `num_fat_sectors > 0`.
- `num_clusters >= EXFAT_RESERVED_CLUSTERS`.
- Valid data-cluster identifiers for later components are in the inclusive range `EXFAT_RESERVED_CLUSTERS..=num_clusters`.
- `root_dir` is within the valid data-cluster identifier range.
- `dentries_per_clu == cluster_size / 32`.
- `vol_flags_persistent` contains only the persistent subset `VOLUME_DIRTY | MEDIA_FAILURE`.
- `cluster_search_ptr == EXFAT_FIRST_CLUSTER`.
- `used_clusters == !0` means cluster usage is not known yet and must be filled by a later bitmap component.

## Concurrency Specification

- Shared state:
  - The only shared object is the borrowed `block_device`.
  - `ExfatBootSector` and `ExfatSuperBlock` are immutable values after construction.
- Locking or serialization assumptions:
  - This component introduces no locks and requires none of its own.
  - The caller is expected to run mount bootstrap against a stable device view; concurrent external mutation of the underlying volume is outside this component's guarantees.
- Required atomicity:
  - Each sector read must be individually atomic at the block-device interface level.
  - The full multi-sector checksum verification is not globally atomic across the device; the component assumes the boot region is not changing during mount.
- Forbidden interleavings:
  - The creator must not add shared mutable caches or global warning state to this component.
  - The creator must not read the Backup Boot Region as part of this component's control flow.
- Behavior under concurrent readers or writers:
  - Concurrent readers are allowed if the block-device implementation supports them.
  - Concurrent writers to the same volume are treated as unsupported mount-time interference; behavior is unspecified beyond surfaced I/O or validation errors.

## Tests and Observability

- Checker-owned unit or kernel tests expected:
  - A success-path `#[ktest]` that reads the embedded exFAT image through the existing in-memory block-device pattern and confirms `read_primary_super_block` succeeds.
  - A targeted `#[ktest]` that corrupts boot-sector signature and expects validation failure.
  - A targeted `#[ktest]` that corrupts `fs_name` and expects validation failure.
  - A targeted `#[ktest]` that corrupts one byte in `must_be_zero` and expects validation failure.
  - A targeted `#[ktest]` that corrupts one byte in the checksummed region and expects checksum verification failure.
  - A targeted `#[ktest]` that corrupts the checksum sector and expects checksum verification failure.
  - At least one geometry-consistency test that rejects impossible FAT or data-region placement.
- Observable behaviors the checker should verify:
  - The normalized `ExfatSuperBlock` fields match the formulas above on the known-good image.
  - Invalid boot metadata is rejected before any later bootstrap step is reachable.
  - Dirty or media-failure flags are preserved in normalized state, but this component does not mutate them and does not emit policy logs.
  - The test commands can be run as filtered kernel tests, and the checker records KVM-versus-TCG observations per the testing guide.

## Creator Notes

The creator must not silently reinterpret the following constraints:

- Keep this component read-only.
- Keep backup-boot fallback and recovery policy out of scope.
- Ignore the checker-owned `#[ktest]` obligations above. The creator should implement production code only unless the main agent explicitly overrides the default workflow.
- Do not collapse boot parsing, checksum verification, and superblock normalization back into a monolithic `fs.rs`.
- Preserve legacy-comparable type names `ExfatBootSector` and `ExfatSuperBlock`.
- Use narrow visibility by default and keep later mount-policy behavior out of this component.
- If a validation rule seems to require later mount policy to decide, stop and surface it to the main agent instead of embedding ad hoc behavior.
