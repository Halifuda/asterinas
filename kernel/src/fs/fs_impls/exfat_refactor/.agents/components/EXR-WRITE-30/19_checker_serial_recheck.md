<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report

## Metadata

- Component ID: `EXR-WRITE-30`
- Title: control-run and pre-test instrumentation recheck for `W30-K3`
- Status: `Blocked`
- Author: checker
- Date: `2026-04-13`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-2052-checker-async-supplement-packet.md`
- Checked implementation: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- Pass kind: `serial recheck`

## Scope of Review

Continued the blocked `W30-K3` checker loop with two user-requested discriminators:

- run an exact-name control ktest unrelated to `publication_gate`
- add temporary stage markers inside the lock-related carrier ktest to see whether guest execution reaches the test body at all

This recheck stayed within the `inode.rs` checker write set and removed the temporary test instrumentation after collecting the evidence.

## Verified Properties

- `/dev/kvm` is still visible in the container, but the actual runs continue to emit TCG warnings.
- The lock-related exact-name rerun still rebuilt successfully after the earlier `RwMutex<()>` repair.
- The control exact-name ktest `boot_policy_publishes_before_root_open_and_stays_stable` stalled during early guest boot within the observed window, so the environment is still unstable outside `EXR-WRITE-30` too.
- In the first instrumented carrier rerun, `/home/halifuda/asterinas/qemu-serial.log` showed:
  - `before first read_at`
  - `after first read_at`
  - `before write_at`
- That same run never emitted `after write_at`, so at least one successful guest boot reached the test body and then stalled inside `write_at`.

## Findings

### Finding

- Severity: Blocking mixed runtime failure
- Location: Docker + QEMU execution environment plus `inode.rs` write path
- Description: The blocker is no longer just a generic boot failure, but it is not a cleanly isolated logic bug either. One lock-unrelated control ktest still stalled during early guest boot, showing real environment instability. However, the instrumented carrier rerun also proved that at least one successful guest boot reached the ktest body and then stalled after `before write_at` without ever printing `after write_at`. That means `W30-K3` still carries a surviving write-path hang once the guest gets far enough to exercise the row.
- Violated spec clause or expected behavior: `W30-K3` needs executable proof that the publication seam preserves buffered-write progress. The current evidence instead shows unstable guest startup plus a surviving stall during `write_at` when the guest does reach the test body.
- Reproduction or reasoning:
  - `cargo osdk test inode_carrier_snapshots_metadata_and_exercises_buffered_read` rebuilt and launched QEMU, then stalled during early boot
  - `cargo osdk test boot_policy_publishes_before_root_open_and_stays_stable` rebuilt and launched QEMU, then also stalled during early boot within the observed window
  - `rg -n "\\[exfat write30 debug\\]" /home/halifuda/asterinas/qemu-serial.log` showed `before first read_at`, `after first read_at`, and `before write_at`, but no `after write_at`

## Unverified Properties

- I still did not obtain clean executable proof for any of the `W30-K3` exact-name obligations.
- I still do not have a clean executable pass for the full `W30-K3` proof set.
- I do not yet know which substep inside `write_at` is hanging once the instrumented carrier rerun reaches that method, because the deeper follow-up rerun did not reproduce the same guest progress within the observed window.

## Recommendation

- Next owner: `main-agent`
- Reason: treat the remaining blocker as mixed. The environment is unstable, but the strongest positive signal now points at a surviving stall inside `write_at` once the guest reaches the carrier test body. The next useful step is a fresh environment-stable rerun with narrower write-path tracing rather than a blind new owner split or unrelated creator work.
- Blocking or non-blocking: blocking for `W30-K3`
- If this is the last checker pass before acceptance, state whether it was a required final checker or a previously recorded skip case: this is still the required serial checker loop for the async supplement, and executable verification remains incomplete.

## Command Evidence

- `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
- `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_carrier_snapshots_metadata_and_exercises_buffered_read'`
- `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test boot_policy_publishes_before_root_open_and_stays_stable'`
- `tail -n 160 /home/halifuda/asterinas/qemu-serial.log`
- `tail -n 120 /home/halifuda/asterinas/qemu.log`
- `rg -n "\\[exfat write30 debug\\]" /home/halifuda/asterinas/qemu-serial.log`
- `docker exec codex-asterinas-dev bash -lc 'ps -ef | grep qemu-system-x86_64 | grep -v grep'`
- `docker exec codex-asterinas-dev bash -lc 'kill 197'`
- `docker exec codex-asterinas-dev bash -lc 'kill 405'`
- `docker exec codex-asterinas-dev bash -lc 'kill 883'`
