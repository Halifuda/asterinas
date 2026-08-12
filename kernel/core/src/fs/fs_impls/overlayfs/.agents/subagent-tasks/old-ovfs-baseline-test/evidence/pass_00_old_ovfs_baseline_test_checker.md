<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Pass Validation Report: old-ovfs-baseline-test

**Status:** COMPLETE / BASELINE EVIDENCE ONLY

This receipt records one fresh 8 GiB TEST/SCRATCH image pair per isolated case
and one QEMU per target from `full.list`. It is not acceptance evidence for
the overlayfs refactor.

## Pass Identity

- **Checker Pass ID:** `pass_00_old_ovfs_baseline_test`
- **Pass Kind:** Meso Integration Pass (pre-design legacy baseline exception)
- **Parent Meso-Component:** `legacy_overlayfs_baseline_validation`
- **Covered Micro-Features:** Packet-declared P0/P1/P2/P3 feature set.

## Execution Record

- **Canonical command:**

      docker exec -w /root/asterinas codex-asterinas-dev make run_kernel AUTO_TEST=conformance RELEASE=1 MEM=12G CONFORMANCE_TEST_SUITE=xfstests XFSTESTS_FS_TYPE=overlay XFSTESTS_DISK_SIZE=8G XFSTESTS_RUNLIST=/opt/xfstests/overlay/run_list/baseline-single.list

- **Targets:** 80 cases from `test/initramfs/src/conformance/xfstests/overlay/run_list/full.list`.
- **Classification source:** The current case's host `qemu.log` only. `qemu-serial.log`, guest results, e2fsck, mount state, image health, and command exit status did not override it.
- **Image isolation:** Before every case, the exact generated TEST/SCRATCH files were removed as root in `codex-asterinas-dev`, verified absent, and recreated by the canonical command. Every observed recreated image was `8589934592` bytes; the first pair was verified as ext2.
- **Timeout:** 300 seconds per case, with QEMU/make descendant cleanup before the next case.
- **CORRUPT:** None permitted or recorded in this rerun.
- **Harness block:** None. The one-hour controller command ended after launching `overlay/076`, but its QEMU completed with a uniquely attributable `qemu.log`; after lane teardown, `overlay/076 NOTRUN` was reconciled from that log before continuing with `077`, `078`, `100`, and `101`.

## Complete Matrix

The durable matrix is
`baseline_case_matrix_rerun.tsv` in this evidence directory; it contains exactly one row for every
target. The actual appended rows, grouped without changing their order, are:

- **PASS (9):** `002`, `007`, `009`, `010`, `016`, `019`, `028`, `039`, `061`
- **FAIL (15):** `003`, `006`, `012`, `013`, `014`, `021`, `022`, `024`, `029`, `031`, `063`, `066`, `067`, `072`, `077`
- **NOTRUN (49):** `001`, `004`, `005`, `008`, `015`, `017`, `018`, `020`, `023`, `025`, `027`, `030`, `032`, `033`, `034`, `035`, `036`, `037`, `040`, `042`, `043`, `044`, `047`, `048`, `049`, `050`, `051`, `052`, `053`, `054`, `055`, `057`, `058`, `059`, `060`, `062`, `064`, `065`, `068`, `069`, `070`, `071`, `073`, `074`, `075`, `076`, `078`, `100`, `101`
- **HANG/TIMEOUT (7):** `011`, `026`, `038`, `041`, `045`, `046`, `056`

Case identifiers in the grouped list are `overlay/<id>` in the TSV.

## Failure and Timeout Receipt

- **Reproduce Command:** The canonical command above, with the single-case
  runlist set to the listed case.
- **Failed Test:** The `FAIL` rows are explicit xfstests failure/output
  results in their case-specific `qemu.log`; the `HANG/TIMEOUT` rows reached
  the 300-second wall-clock cutoff without a terminal result.
- **Evidence:** Compact status rows are preserved in
  `baseline_case_matrix_rerun.tsv` in this evidence directory. No per-case logs or full per-case command
  copies were retained.

## Cleanup

The packet-created `baseline-single.list` was removed after the final case.
The Checker modified no production code, filesystem-local tests, official
runlists, or main-agent handoffs.
