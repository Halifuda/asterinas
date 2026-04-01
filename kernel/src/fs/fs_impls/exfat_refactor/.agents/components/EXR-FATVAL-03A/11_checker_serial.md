<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Log

## Metadata

- Component ID: EXR-FATVAL-03A
- Title: FAT Entry Value Model And Single-Step Next-Cluster Decode
- Phase: `SerialChecker`
- Author: checker
- Date: 2026-04-01
- Validated creator artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FATVAL-03A/10_creator_serial.md`

## Scope

- Checked the serial creator pass for the typed FAT value model and the one-step read-only decode helper.
- Added the smallest checker-owned ktests needed to cover raw conversion, on-disk decode, invalid source rejection, and invalid next-cluster rejection.
- Stopped after this checker artifact, per packet boundary.

## Code Changes

- Added checker-owned ktests in `kernel/src/fs/fs_impls/exfat_refactor/fat.rs` for:
  - special FAT raw-value mapping and reverse conversion,
  - on-disk decode against the embedded exFAT image,
  - invalid source-cluster rejection,
  - invalid decoded next-cluster rejection.

## Verification

- KVM preflight:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - Observation: `no-kvm`
  - Result: QEMU runs used TCG fallback, not KVM.
- Test command 1:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test fat_value_preserves_special_markers_and_next_clusters'`
  - Observation: build completed, QEMU booted under TCG, test passed.
- Test command 2:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test read_next_fat_value_decodes_embedded_image_entry'`
  - Observation: build completed, QEMU booted under TCG, test passed.
- Test command 3:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test read_next_fat_value_rejects_invalid_source_cluster'`
  - Observation: build completed, QEMU booted under TCG, test passed.
- Test command 4:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test read_next_fat_value_rejects_invalid_next_cluster_target'`
  - Observation: build completed, QEMU booted under TCG, test passed.

## Result

- The serial creator pass is validated.
- The checker pass is non-blocking.
- No production-code fix was required.

## Notes

- All test runs emitted standard TCG CPU-feature warnings, but no build or test failures.
- The checked behavior matches the spec:
  - raw FAT markers remain distinct and reversible,
  - on-disk reads use the shared metadata I/O path,
  - invalid source clusters and invalid decoded next-cluster targets are rejected.
