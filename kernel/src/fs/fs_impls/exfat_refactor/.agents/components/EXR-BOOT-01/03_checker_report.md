<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report

## Metadata

- Component ID: EXR-BOOT-01
- Title: Boot Region Parsing And Normalized Runtime Geometry
- Status: `Checked`
- Author: checker
- Date: 2026-03-31
- Checked implementation: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-01/02_creator_log.md`

## Scope of Review

Checked the `EXR-BOOT-01` production implementation in:

- `kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`

Reviewed the component against:

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-01/01_designer_spec.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-01/00_architect.md`

Validation work included:

- static inspection of the validation, checksum, normalization, and ktest code,
- environment verification for KVM versus TCG,
- rerunning the creator's filtered ktest,
- rerunning the legacy `new_exfat` ktest as a control in the same container.

## Test Changes

None.

## Findings

### Finding

- Severity: High
- Location: `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- Description: The success-path ktest for `EXR-BOOT-01` is not currently an executable confirmation of the component contract. In the validated `codex-asterinas-dev` container with `no-kvm`, `cargo osdk test boot_region_loads_super_block` and `cargo osdk test exfat_refactor::tests::boot_region_loads_super_block` both boot QEMU and then exit with status `1`, with output stopping around UEFI boot and including `error: no suitable video mode`. In the same environment, the legacy control `cargo osdk test new_exfat` exits `0`.
- Violated spec clause or expected behavior: `01_designer_spec.md` requires a success-path `#[ktest]` that confirms `read_primary_super_block` succeeds on the embedded image, and the checker must be able to verify the observable behavior through executable checks.
- Reproduction or reasoning:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'` returned `no-kvm`.
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test boot_region_loads_super_block'` exited `1`.
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test boot_region_rejects_invalid_signature'` exited `0` in the same container, so the new `exfat_refactor` ktest path is not failing uniformly.
  - The observed run used TCG and printed `error: no suitable video mode` after the UEFI boot messages.
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test new_exfat'` exited `0` in the same container, so the current blocker is not explained by TCG alone.
  - This is a blocking verification failure localized to the success-path ktest or the code it exercises. The current evidence no longer supports treating it as a generic runner failure.

### Finding

- Severity: Low
- Location: `kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`
- Description: The file-level `#[expect(dead_code)]` in `boot_sector.rs` is now unfulfilled, so the component emits a warning during `cargo osdk test` even though these helpers are actively referenced by the ktests.
- Violated spec clause or expected behavior: Repository lint hygiene under the root `AGENTS.md` prefers narrow, accurate lint suppression.
- Reproduction or reasoning:
  - The `cargo osdk test` build prints: `warning: this lint expectation is unfulfilled` for `boot_sector.rs`.
  - This does not block functionality, but it is noise that should be cleaned up in a follow-up repair batch.

## Verified Properties

- `read_primary_super_block` follows the specified call order: read primary boot sector, validate it, verify the primary boot-region checksum, then normalize to `ExfatSuperBlock`.
- `validate_primary_boot_sector` covers the required primary-boot checks from the accepted designer spec: signature, FS name, zeroed reserved bytes, FAT count, sector-size bounds, cluster-size bound, FAT offset, FAT length, cluster count, root-cluster range, FAT capacity, data-region placement, and volume length.
- `verify_primary_boot_region_checksum` reads only the primary boot region, skips bytes `106`, `107`, and `112`, and compares every checksum entry in sector `11`.
- `ExfatSuperBlock` normalization preserves the required formulas for sector size, cluster size, FAT starts, data start, root cluster, persistent volume flags, cluster search pointer, and unknown used-cluster count.
- The component stays read-only and does not widen into `ExfatFs`, inode, bitmap, upcase, or registration work.
- `make kernel` succeeded in the creator's recorded self-checks.
- The current observed runtime mode is TCG, not KVM.

## Unverified Properties

- The success-path ktest has not yet provided a reliable executable confirmation of `read_primary_super_block` on the embedded image.
- The malformed-input ktests were not accepted as verified coverage because the success-path ktest itself is currently not trustworthy as a stable runner path.
- The precise cause of the success-path failure is still unverified: it may be a logic error in `read_primary_super_block`, a normalization mismatch exposed by `assert_super_block_matches_boot_sector`, or another success-path-only defect.

## Recommendation

- Next owner: `advisor`
- Reason: Negative-path ktests execute successfully in the same environment, so the next step should focus on the success-path bootstrap or assertion mismatch instead of treating this as a generic runner problem. The stale lint expectation should be cleaned up in the same repair batch.
- Blocking or non-blocking: Blocking
