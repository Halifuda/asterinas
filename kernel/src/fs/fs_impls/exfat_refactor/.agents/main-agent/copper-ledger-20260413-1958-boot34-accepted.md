<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff

## Metadata

- Fancy nickname: `copper-ledger`
- Date: `2026-04-13 19:58 CST`
- Covered hours: `EXR-BOOT-34` closure after the `EXR-RESIZE-37` split, plus scheduler-state cleanup
- Author: `main-agent`
- Workspace: `/home/halifuda/asterinas`
- Container or environment: shared host workspace plus shared Docker container `codex-asterinas-dev`
- Status: `EXR-BOOT-34` is now accepted; `EXR-RESIZE-37` remains the newly explicit missing resize/truncate module; the user has chosen to reopen `EXR-WRITE-30` at its queued async supplement, the packet has been refreshed to post-split reality, and the creator lane is currently running with no `16_creator_serial_repair.md` artifact yet at this checkpoint.

## Environment Summary

- Image or base environment:
  - shared host workspace plus `codex-asterinas-dev`
- Working path:
  - `/home/halifuda/asterinas`
- Container name, if any:
  - `codex-asterinas-dev`
- KVM status:
  - `/dev/kvm` is visible in the container, but the `EXR-BOOT-34` checker recorded TCG fallback during the successful exact-name run
- Validated commands:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test boot_policy_publishes_before_root_open_and_stays_stable && cargo osdk test boot_policy_dirty_intent_stays_separate_from_trusted_source && cargo osdk test boot_policy_percent_in_use_is_observational_only'`
- Known environment blockers:
  - no active checker lock at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/locks/checker-execution.lock/`

## Current Project State

- Current goal:
  - keep the board honest after the resize split and the `EXR-BOOT-34` closure, then execute the user-approved `EXR-WRITE-30` async supplement without reopening stale resize context
- Current phase:
  - post-`EXR-BOOT-34` continuity update plus `EXR-WRITE-30` async-supplement restart
- Active or next component:
  - `EXR-WRITE-30`
  - the refreshed async-supplement creator packet has now been dispatched and owns only `inode.rs` plus its creator artifact
- Latest accepted components:
  - accepted rows now include `EXR-BOOT-34`
  - `EXR-CHARSET-32` remains accepted
- Components in progress:
  - `EXR-WRITE-30` remains `SerialImplementing` for its async supplement only
  - `EXR-NAMESPACE-29` and `EXR-SYNC-31` remain `Specified`
- Blocked components:
  - `EXR-RESIZE-37` is still planning-blocked on an explicit filesystem-owner release/reclaim decision
  - `EXR-NAMESPACE-29` still should not overtake `EXR-WRITE-30` because both collide in `inode.rs`

## Recent Decisions

- Kept the resize split: `EXR-WRITE-30/14_creator_serial_repair.md` remains the authoritative proof that shrink/truncate needs a separate row and owner decision.
- Chose `EXR-BOOT-34` as the lowest-cost ready specified row and ran it through creator, checker, and reviewer instead of reopening `inode.rs`.
- Recorded the `EXR-BOOT-34` checker result at:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/11_checker_serial.md`
  - Outcome: three exact-name local `fs.rs` regressions passed; `/dev/kvm` existed, but the actual guest run used TCG.
- Recorded the `EXR-BOOT-34` reviewer result at:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/30_reviewer_report.md`
  - Outcome: no reviewer findings; no production edits were made in the reviewer lane.
- Updated `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md` so `EXR-BOOT-34` is now accepted and the board explicitly keeps `EXR-RESIZE-37` as the separate missing module.
- The user explicitly chose option `2` rather than opening a new `EXR-RESIZE-37` architect/designer loop first.
- Before dispatching the queued `EXR-WRITE-30` async supplement, the main agent refreshed `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-1921-creator-async-supplement-packet.md` so it matches the post-split board reality:
  - no dependency on a serial resize landing
  - no `resize` or `fs.rs` ownership in scope
  - explicit treatment of `EXR-RESIZE-37` as out of scope
- The refreshed `EXR-WRITE-30` async supplement packet has now been dispatched as the active creator lane.
  - Write set: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - Creator artifact target: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/16_creator_serial_repair.md`
  - Stop rule remains active: if the serialization fix still needs resize/truncate semantics or a new owner outside `ExfatInode`, record that exact stop condition instead of widening scope.
- Checkpoint note:
  - At the time of this handoff update, the creator lane is still running and `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/16_creator_serial_repair.md` does not exist yet.

## Open Risks And Assumptions

- `EXR-WRITE-30` still has a queued async supplement driven by the already-written `02_designer_async.md`; this is a same-row implementation follow-up, not automatic proof that a brand-new architect/designer component is needed.
- The active `EXR-WRITE-30` async supplement is intentionally scoped to buffered writes only.
  - If the creator reports that closing the serialization risk still requires resize/truncate semantics or a new owner outside `ExfatInode`, that is a real stop signal and should be recorded as such rather than patched around.
- The user previously asked for write-path async concerns to be surfaced before opening any brand-new architect/designer loop.
  - Because `EXR-WRITE-30` already has async designer authority, the next async step can stay inside that row unless new evidence forces a broader owner split.
- `EXR-RESIZE-37` remains the explicit missing module for regular-file `resize`/truncate.
  - The unresolved seam is a filesystem-owner release/reclaim handshake symmetric to `ExfatFs::allocate_clusters()`.
  - That seam must keep FAT tail teardown, allocation-bitmap release, and persistent free-space publication coherent.
- `EXR-WRITE-30`, `EXR-RESIZE-37`, `EXR-NAMESPACE-29`, `EXR-DIRECT-33`, and `EXR-INODE-META-36` all collide in `inode.rs`; do not open file-parallel creator lanes across those rows.
- `EXR-SYNC-31` and `EXR-VOLLABEL-35` still collide in `fs.rs`; keep those creator waves serialized too.
- Command-free lanes that appear safe in principle but remain intentionally unstarted for quota reasons:
  - `EXR-RESIZE-37` architect or designer work
  - `EXR-VOLLABEL-35` architect or designer work
  - `EXR-INODE-META-36` architect or designer work

## Recommended Next Actions

1. Wait for the active `EXR-WRITE-30` async-supplement creator lane to either land a bounded `inode.rs` serialization seam or stop on a packet-defined owner gap.
2. If that creator lands cleanly, record the result immediately in a fresh main-agent handoff before opening `W30-K3`.
3. If that creator stops on a broader owner gap, record the exact stop condition immediately in the current handoff before any new row or packet is opened.

## Resume Checklist

- Use `$exfat-main-agent`.
- Read `README.md`.
- Read `COMPONENT_INDEX.md`.
- Read this handoff after `saffron-ledger-20260413-1906-write30-state-rebuild.md`.
- Treat `EXR-BOOT-34` as fully closed for now:
  - creator: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/10_creator_serial.md`
  - checker: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/11_checker_serial.md`
  - reviewer: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/30_reviewer_report.md`
- Treat `EXR-RESIZE-37` as the separate missing module created from the stopped `EXR-WRITE-30` resize attempt.
- Treat `EXR-WRITE-30` as open only for its queued buffered-write async supplement, not for another hidden resize retry.
- Treat `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-1921-creator-async-supplement-packet.md` as refreshed to post-split reality before dispatch.
- Do not assume any checker lock is live; verify `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/locks/checker-execution.lock/owner.toml` before any new checker lane.
