<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report

## Metadata

- Component ID: EXR-INODE-CACHE-18
- Title: ExfatFs Opened-Inode Table And Validated InodeKey
- Status: `SerialChecked`
- Author: checker
- Date: 2026-04-10
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CACHE-18/20260410-1140-checker-retry-packet.md`
- Checked implementation: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- Pass kind: `serial`

## Scope of Review

Re-checked the repaired `fs.rs` cache boundary against `01_designer_core.md`, `02_designer_async.md`, `03_designer_ktest.md`, `10_creator_serial.md`, `11_checker_serial.md`, `12_creator_serial_repair.md`, and the local `ExfatFs` / `ExfatInode` integration surface. The retry focused on executable proof for the three checker-owned cache regressions in `fs.rs`.

## Test Changes

No new tests were added in this retry. The three checker-owned `#[ktest]` cases remained in `fs.rs`:

- `inode_key_tracks_only_trusted_location_facts`
- `opened_inode_state_reuses_canonical_handle_and_exact_key_removal`
- `root_special_case_stays_outside_the_ordinary_keyspace`

Source-backed suffix proof remains in `fs.rs`:

- `rg -n "inode_key_tracks_only_trusted_location_facts|opened_inode_state_reuses_canonical_handle_and_exact_key_removal|root_special_case_stays_outside_the_ordinary_keyspace" /home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- Result lines: `330`, `342`, and `364`

## Findings

No findings.

## Verified Properties

- `/dev/kvm` is present in the execution container: `crw-rw---- 1 root 109 10, 232 Apr 10 01:57 /dev/kvm`
- The repaired `fs.rs` compiled far enough for the checker-owned ktests to execute.
- `InodeKey` remains derived only from trusted location facts.
- The opened-inode table reuses a canonical handle and supports exact-key removal.
- The root special case remains outside the ordinary keyed table.
- All three exact filtered ktest commands completed successfully with `exit 0`.
- QEMU emitted `TCG` capability warnings during the guest runs, so the recorded runtime evidence came from a TCG-backed guest path despite `/dev/kvm` being visible on the host.

## Unverified Properties

- None within the retry scope.

## Recommendation

- Next owner: main agent
- Reason: The retry produced executable evidence for the three checker-owned cache regressions, and no additional `fs.rs`-local failure surfaced.
- Blocking or non-blocking: Non-blocking
- If this is the last checker pass before acceptance, state whether it was a required final checker or a previously recorded skip case: Required retry checker pass before acceptance.
