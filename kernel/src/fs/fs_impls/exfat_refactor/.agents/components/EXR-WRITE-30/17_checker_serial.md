<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report

## Metadata

- Component ID: `EXR-WRITE-30`
- Title: `ExfatInode` buffered write, committed growth, and publication-gate supplement
- Status: `SerialChecked`
- Author: checker
- Date: `2026-04-13`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-2052-checker-async-supplement-packet.md`
- Checked implementation: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- Pass kind: `serial`

## Scope of Review

Checked the landed `EXR-WRITE-30` async supplement in `inode.rs`, focusing on the owner-private `publication_gate` and whether the earlier buffered-write slice still stays within one inode-local published state. This pass stayed inside `inode.rs`, did not reopen resize/truncate ownership, and did not touch direct I/O or sync ordering.

## Test Changes

- Added one compact checker-owned `#[ktest]` in `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`:
  - `inode_publication_gate_keeps_read_and_npages_on_one_published_state`
- The new test is scenario-labeled and stays narrowly focused on buffered-write publication rather than resize/truncate behavior.
- Made one strictly local production fix in `inode.rs`: `PageCacheBackend::npages()` now prefers the publication-gate read path but falls back to the committed inode snapshot if page-cache internals ask for `npages()` while the same inode already owns the write side of the publication gate.

## Findings

### Finding

- Severity: High
- Location: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs:1009`
- Description: The landed `publication_gate` wrapped `PageCacheBackend::npages()` in an unconditional read lock, which can self-deadlock when page-cache internals ask the inode backend for `npages()` while the same buffered-write call already owns the write side of `publication_gate`. I repaired this locally by falling back to the committed inode snapshot when `try_read()` cannot acquire the gate.
- Violated spec clause or expected behavior: The async supplement is supposed to preserve one owner-local publication seam for buffered writes, not hang the write path before publication completes.
- Reproduction or reasoning: The initial exact-name checker run reached `inode_carrier_snapshots_metadata_and_exercises_buffered_read` in `qemu-serial.log` and then stopped making forward progress. The repaired `npages()` path avoids re-entering the same owner-local seam from page-cache internals.

## Verified Properties

- `/dev/kvm` is visible in the container, but the attempted `cargo osdk test` runs emitted `qemu-system-x86_64` TCG warnings, so executable proof was using TCG rather than KVM.
- The exact-name filter for `inode_carrier_snapshots_metadata_and_exercises_buffered_read` hit the intended `#[ktest]`; the guest serial log printed that test name explicitly before the initial hang.
- The publication-seam repair stayed strictly local to `inode.rs`.
- The new publication-focused checker test now exists in `inode.rs` and compiled as part the successful kernel builds for the attempted checker runs.

## Unverified Properties

- I did not obtain a clean post-repair executable pass for:
  - `inode_carrier_snapshots_metadata_and_exercises_buffered_read`
  - `inode_buffered_write_grows_an_empty_file_and_publishes_allocation_facts`
  - `inode_buffered_write_extends_a_non_empty_file_across_growth`
  - `inode_publication_gate_keeps_read_and_npages_on_one_published_state`
- After the local `npages()` repair, the first rerun was blocked by a stale QEMU process still holding the ext2 image lock from the earlier hung run; after cleanup, a fresh rerun booted QEMU again but did not progress past early guest boot output within the observed window, and neither `qemu-serial.log` nor `qemu.log` surfaced a guest-side panic site.
- Resize/truncate publication remains intentionally out of scope for this packet and was not revalidated here.

## Recommendation

- Next owner: `main-agent`
- Reason: the strictly local `inode.rs` publication-seam bug is repaired, but runtime proof is still incomplete and needs a fresh clean checker rerun or environment triage before the async supplement can be considered fully checked.
- Blocking or non-blocking: blocking
- If this is the last checker pass before acceptance, state whether it was a required final checker or a previously recorded skip case: this was a required serial checker pass, but executable proof remains incomplete after the in-scope repair.

## Command Evidence

- `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
- `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_carrier_snapshots_metadata_and_exercises_buffered_read && cargo osdk test inode_buffered_write_grows_an_empty_file_and_publishes_allocation_facts && cargo osdk test inode_buffered_write_extends_a_non_empty_file_across_growth && cargo osdk test inode_publication_gate_keeps_read_and_npages_on_one_published_state'`
- `tail -n 120 /home/halifuda/asterinas/qemu-serial.log`
- `tail -n 80 /home/halifuda/asterinas/qemu.log`
- `docker exec codex-asterinas-dev bash -lc 'ps -ef | grep -E "qemu-system-x86_64|cargo osdk test inode_" | grep -v grep'`
- `docker exec codex-asterinas-dev bash -lc 'kill 17224 16779'`
- `docker exec codex-asterinas-dev bash -lc 'kill 18401 18225'`
- `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_carrier_snapshots_metadata_and_exercises_buffered_read'`
