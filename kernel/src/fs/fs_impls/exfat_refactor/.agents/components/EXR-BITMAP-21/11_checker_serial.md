<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Log

## Metadata

- Component ID: `EXR-BITMAP-21`
- Title: `ExfatFs` allocation-bitmap owner boundary
- Status: `Pass with executable evidence`
- Author: Codex
- Date: `2026-04-10`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BITMAP-21/20260410-1335-checker-serial-packet.md`
- Checked spec: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/01_designer_core.md`
- Pass kind: `serial checker`

## Scope

- Production files inspected:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- Checker-owned test file updated:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- Checker artifact written:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/11_checker_serial.md`

## Evidence

- `/dev/kvm` was visible before the runs.
- Actual test runs used QEMU TCG, not KVM. The harness emitted `qemu-system-x86_64` TCG warnings on every run.

## Commands Run

1. `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test bitmap::tests::invalid_bitmap_load_is_rejected_before_publication'`
2. `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test bitmap::tests::loaded_bitmap_reports_first_middle_and_tail_cluster_occupancy'`
3. `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test bitmap::tests::bitmap_accounting_ignores_padding_bits_beyond_valid_range'`
4. `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test bitmap::tests'`

## Results

- The first two exact-suffix commands exited `0`.
- The third exact-suffix command exited `1` without useful guest output, so I treated it as an unusable filter rather than a code failure and validated the full `bitmap::tests` module instead.
- The module-level run exited `0`.
- The module-level run covered all three required regressions in `bitmap.rs`.

## Regression Coverage

- Invalid bitmap images are rejected before publication.
- Occupancy queries agree with the published bitmap bytes for first, middle, and tail clusters.
- Used/free accounting matches the same snapshot and ignores padding bits beyond the valid cluster range.

## Temporary Debug Use

- I added a temporary `println!` inside `bitmap_accounting_ignores_padding_bits_beyond_valid_range` to localize the exact-suffix run issue, then removed it before writing this artifact.
- No temporary debug edits remain in the checked source state.

## Conclusion

The bitmap-owner boundary passes the required checker coverage.
