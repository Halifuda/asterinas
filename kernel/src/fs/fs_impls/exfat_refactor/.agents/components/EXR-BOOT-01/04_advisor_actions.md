<!-- SPDX-License-Identifier: MPL-2.0 -->

# Advisor Actions

## Metadata

- Component ID: EXR-BOOT-01
- Title: Boot Region Parsing And Normalized Runtime Geometry
- Status: `Advised`
- Author: advisor
- Date: 2026-03-31
- Based on checker artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-01/03_checker_report.md`

## Repair Plan

1. Change:
   Diagnose and fix the success-path failure in `EXR-BOOT-01`. The repair may touch `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`, `kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`, and `kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`, but it must stay confined to the success-path bootstrap and its assertion surface.
   Reason:
   The component cannot advance while `cargo osdk test boot_region_loads_super_block` exits `1` under TCG. A negative-path control in the same `exfat_refactor` module, `cargo osdk test boot_region_rejects_invalid_signature`, exits `0`, so the blocker is now localized to the success-path code or assertion rather than a generic runner failure.
   Source:
   `03_checker_report.md` finding 1.
   Done when:
   In `codex-asterinas-dev` with `no-kvm`, both `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test boot_region_loads_super_block'` and `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test exfat_refactor::tests::boot_region_loads_super_block'` complete with exit code `0`, and the creator records the concrete root cause of the previous failure.

2. Change:
   Remove or narrow the stale `#[expect(dead_code)]` suppression in `kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs` so the component no longer emits an unfulfilled lint expectation during `cargo osdk test`.
   Reason:
   The warning is no longer accurate because the helpers are actively referenced by the staged tests, and keeping stale lint suppressions will add noise to later verification passes.
   Source:
   `03_checker_report.md` finding 2.
   Done when:
   A rerun of the filtered `EXR-BOOT-01` test build no longer emits the `unfulfilled_lint_expectations` warning for `boot_sector.rs`.

## Deferred Issues

- Do not widen this repair batch into backup boot-region fallback, `ExfatFs` integration, or mount-policy changes. None of those are needed to resolve the current blocker.
- Do not widen this repair batch beyond the success-path bootstrap, its assertion surface, and the stale lint expectation. Negative-path tests already execute in the current environment and should be used only as controls.

## Retest Plan

The checker should rerun:

- `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
- `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas && make kernel'`
- `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test boot_region_loads_super_block'`
- `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test exfat_refactor::tests::boot_region_loads_super_block'`

If the repaired success-path test passes, the checker should then rerun at least one malformed-input ktest from `mod.rs` to confirm the negative path still executes under the same runner conditions.
