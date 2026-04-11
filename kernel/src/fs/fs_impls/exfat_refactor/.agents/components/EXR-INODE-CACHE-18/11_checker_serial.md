<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report

## Metadata

- Component ID: EXR-INODE-CACHE-18
- Title: ExfatFs Opened-Inode Table And Validated InodeKey
- Status: `SerialChecked`
- Author: checker
- Date: 2026-04-10
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CACHE-18/20260410-1112-checker-serial-packet.md`
- Checked implementation: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- Pass kind: `serial`

## Scope of Review

Checked `fs.rs` against `01_designer_core.md`, `02_designer_async.md`, `03_designer_ktest.md`, `10_creator_serial.md`, and the local `ExfatFs` / `ExfatInode` integration surface allowed by the packet.

## Test Changes

Added three local `#[ktest]` cases in `fs.rs`:

- `inode_key_tracks_only_trusted_location_facts`
- `opened_inode_state_reuses_canonical_handle_and_exact_key_removal`
- `root_special_case_stays_outside_the_ordinary_keyspace`

The source-backed suffix proof is in `fs.rs`:

- `rg -n "inode_key_tracks_only_trusted_location_facts|opened_inode_state_reuses_canonical_handle_and_exact_key_removal|root_special_case_stays_outside_the_ordinary_keyspace" /home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- Result lines: `330`, `342`, and `364`

## Findings

### Finding

- Severity: Blocking verification failure
- Location: `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test fs::fs_impls::exfat_refactor::fs::tests::inode_key_tracks_only_trusted_location_facts'`
- Description: The filtered checker run could not reach the new `fs.rs` ktest because compilation failed in unrelated exFAT files outside the write set.
- Evidence:
  - `/dev/kvm` was present: `crw-rw---- 1 root 109 10, 232 Apr 10 01:57 /dev/kvm`
  - The filtered run started QEMU forwarding, then failed during compilation.
  - Reported build errors were in `directory.rs` and `fileset.rs`, including:
    - missing `IntoBytes` import for `RawExfatDentry::as_bytes()`
    - private field access to `ExfatDentrySet::dentries`
    - `DirectoryRecord` lacking `PartialEq` for `assert_eq!`
- Scope note: Those failures are outside the checker write set, so I did not patch them in this pass.

## Verified Properties

- `InodeKey` is owner-private and its constructor takes only directory-location facts.
- `OpenedInodeState` owns the ordinary opened-inode map and a separate root slot.
- The added tests cover key equality/inequality, canonical handle reuse, exact-key removal, and root-slot separation.

## Unverified Properties

- The new checker tests were not executed because crate compilation stopped on unrelated errors before reaching them.
- Canonical handle reuse and root separation are therefore verified only by source inspection in this pass, not by executable evidence.

## Recommendation

- Next owner: main agent
- Reason: Resolve the unrelated exFAT build failures in `directory.rs` / `fileset.rs`, then rerun the filtered checker command under the lock to obtain executable evidence for the new `fs.rs` regressions.
- Blocking or non-blocking: Blocking for checker acceptance evidence, non-blocking for the `fs.rs` source-level review itself.
