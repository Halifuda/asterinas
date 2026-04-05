<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Serial Retry

## Metadata

- Component ID: `EXR-READ-11A`
- Title: `Logical-To-Physical Mapping For Existing Regular-File Reads`
- Status: `SerialChecked`
- Author: `checker`
- Date: `2026-04-05`
- Task packet: `EXR-READ-11A-CHECK-20260405-1148`
- Checked implementation: `kernel/src/fs/fs_impls/exfat_refactor/read.rs`
- Pass kind: `serial`

## Scope of Review

Checked the `read.rs` mapping boundary against the architect and designer artifacts, limited to the checker-owned local `#[ktest]` surface and the mandated serial verification command. The review stayed within the placement-only boundary: contiguous mapping, FAT-backed chain walking, valid-data-length EOF handling, and non-regular-file rejection at the read-view boundary.

## Test Changes

- No checker-owned ktest edits were needed in `read.rs`.
- The required local `#[ktest]` cases were already present in `read.rs` at the expected scope:
  - `contiguous_offset_maps_without_fat_reads`
  - `fat_backed_offset_maps_through_chain`
  - `offset_at_valid_data_end_returns_none`
  - `non_regular_file_is_rejected`
- Each existing test already carries a short scenario comment.

## Runtime Evidence

1. Execution lock acquire command:
   `kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-READ-11A --phase checker-serial-retry --command "docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test contiguous_offset_maps_without_fat_reads && cargo osdk test fat_backed_offset_maps_through_chain && cargo osdk test offset_at_valid_data_end_returns_none && cargo osdk test non_regular_file_is_rejected'" --retry-seconds 60 --wait-budget-seconds 1800`
   - Result: lock acquired.

2. Verification command run under the lock:
   `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test contiguous_offset_maps_without_fat_reads && cargo osdk test fat_backed_offset_maps_through_chain && cargo osdk test offset_at_valid_data_end_returns_none && cargo osdk test non_regular_file_is_rejected'`
   - Result: exited `0`.
   - Runtime mode: TCG-backed QEMU.
   - Observations: each filtered invocation rebuilt the test image, completed ISO generation, and entered QEMU successfully. QEMU emitted the usual TCG unsupported-feature warnings, but no build error, panic, or test failure appeared, and the `&&`-chained command completed successfully.

3. Execution lock release command:
   `kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`
   - Result: `status = "unlocked"`

## Filter-Hit Proof

- The exact suffixes used in the verification command are taken directly from `read.rs`:
  - `contiguous_offset_maps_without_fat_reads` at `read.rs:203`
  - `fat_backed_offset_maps_through_chain` at `read.rs:243`
  - `offset_at_valid_data_end_returns_none` at `read.rs:286`
  - `non_regular_file_is_rejected` at `read.rs:325`
- This is sufficient hit proof because the local ktest runner treats the whitelist as a test-path suffix match:
  - `osdk/deps/test-kernel/src/lib.rs:81-83` states that only tests whose path is the suffix of a whitelisted path run.
  - `osdk/deps/test-kernel/src/lib.rs:136-148` shows the runner building `module_path::fn_name` and checking that suffix before printing and running the test.
  - `ostd/libs/ostd-test/src/lib.rs:240-247` shows OSDK forwarding the generated test whitelist into the runner.
- Within this component scope, these four function names are exact `#[ktest]` suffixes from `read.rs`, so `cargo osdk test <name>` is sufficient to target the intended tests without relying on `exit 0` alone.

## Findings

None.

## Verified Properties

- Contiguous placement remains arithmetic-only at this boundary and does not require block-device reads.
- FAT-backed placement still resolves through the accepted chain-walk path.
- Offsets at or beyond `valid_data_length` return `None`.
- Non-regular-file shells are rejected before the mapper publishes a placement.
- The component remains a placement-only boundary; no buffered-read or page-cache ownership drift was exposed in `read.rs`.

## Unverified Properties

- No downstream caller integration was exercised in this retry pass.
- Buffered `read_at`, page-cache ownership, and write-side extension remain intentionally out of scope for `EXR-READ-11A`.

## Recommendation

- Next owner: `main-agent`
- Reason: the serial checker retry passed with exact filter-hit proof and no in-scope findings.
- Blocking or non-blocking: `Non-blocking`
