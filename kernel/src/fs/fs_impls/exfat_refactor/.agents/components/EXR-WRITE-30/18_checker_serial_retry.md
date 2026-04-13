<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report

## Metadata

- Component ID: `EXR-WRITE-30`
- Title: `ExfatInode` buffered write publication-gate retry after spin-over-I/O diagnosis
- Status: `Blocked`
- Author: checker
- Date: `2026-04-13`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-2052-checker-async-supplement-packet.md`
- Checked implementation: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- Pass kind: `serial retry`

## Scope of Review

Continued the same-row `W30-K3` checker loop after `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/17_checker_serial.md` stopped with incomplete executable proof. The retry stayed inside `inode.rs`, kept resize/truncate ownership under `EXR-RESIZE-37`, and focused on whether the new `publication_gate` shape itself was still invalid before asking the environment for another full proof.

## Findings

### Finding

- Severity: High
- Location: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs:52`
- Description: The async supplement used `ostd::sync::RwLock<()>` for `publication_gate`, but `read_at()` and `write_at()` hold that gate across real metadata I/O, page-cache resizing, and other potentially blocking work. In this repository that lock is spin-based, so the shape violates the lock discipline and is consistent with the earlier guest-side hangs. I repaired this locally by changing the gate to `RwMutex<()>`, which preserves the owner-local publication seam without spinning across blocking work.
- Violated spec clause or expected behavior: The publication seam may serialize buffered visibility, but it must not introduce a spin-based lock around blocking I/O on the inode data path.
- Reproduction or reasoning: Reading `/home/halifuda/asterinas/ostd/src/sync/rwlock.rs` confirmed that `RwLock` is spin-based. `read_at()` and `write_at()` in `inode.rs` keep the gate held while traversing on-disk ranges and page-cache state, so the original `RwLock<()>` shape was unsafe even after the earlier `npages()` self-deadlock repair.

## Test Changes

- No new `#[ktest]` was required in this retry.
- Made one strictly local production fix in `inode.rs`:
  - changed `publication_gate` from `RwLock<()>` to `RwMutex<()>`
- The rerun rebuilt the kernel successfully with that local repair.

## Verified Properties

- `/dev/kvm` is visible in the container.
- The local `inode.rs` repair compiled successfully as part of the rerun build.
- The retry preserved the same owner and write set:
  - `publication_gate` remains owner-private to `ExfatInode`
  - no resize/truncate or filesystem-global coordinator was introduced

## Unverified Properties

- I still did not obtain a clean executable pass for:
  - `inode_carrier_snapshots_metadata_and_exercises_buffered_read`
  - `inode_buffered_write_grows_an_empty_file_and_publishes_allocation_facts`
  - `inode_buffered_write_extends_a_non_empty_file_across_growth`
  - `inode_publication_gate_keeps_read_and_npages_on_one_published_state`
- After the `RwMutex` repair, two fresh exact-name reruns of `inode_carrier_snapshots_metadata_and_exercises_buffered_read` both rebuilt and launched QEMU under TCG, but each stalled during early guest boot.
- In both fresh reruns, `/home/halifuda/asterinas/qemu-serial.log` stopped after:
  - `WARNING: no console will be available to OS`
  - `error: no suitable video mode found.`
- Those reruns never reached the ktest runner within the observed window, so this retry cannot claim executable proof for the local repair.

## Recommendation

- Next owner: `main-agent`
- Reason: the checker closed one more strictly local `inode.rs` bug, but the serial proof set is now blocked on an unstable QEMU/TCG execution environment rather than on a newly identified row-boundary bug.
- Blocking or non-blocking: blocking for `W30-K3`
- If this is the last checker pass before acceptance, state whether it was a required final checker or a previously recorded skip case: this remains the required serial checker loop for the async supplement, and executable verification is still incomplete.

## Command Evidence

- `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
- `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_carrier_snapshots_metadata_and_exercises_buffered_read'`
- `tail -n 160 /home/halifuda/asterinas/qemu-serial.log`
- `tail -n 120 /home/halifuda/asterinas/qemu.log`
- `docker exec codex-asterinas-dev bash -lc 'ps -ef | grep qemu-system-x86_64 | grep -v grep'`
- `docker exec codex-asterinas-dev bash -lc 'kill 19358'`
- `docker exec codex-asterinas-dev bash -lc 'kill 19580'`
