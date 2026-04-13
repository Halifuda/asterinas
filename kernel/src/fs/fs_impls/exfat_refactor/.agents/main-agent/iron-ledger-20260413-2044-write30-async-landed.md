<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff

## Metadata

- Fancy nickname: `iron-ledger`
- Date: `2026-04-13 20:44 CST`
- Covered hours: `EXR-WRITE-30` async-supplement creator landing plus the follow-on checker retry after the earlier `EXR-RESIZE-37` split and `EXR-BOOT-34` closure
- Author: `main-agent`
- Workspace: `/home/halifuda/asterinas`
- Container or environment: shared host workspace plus shared Docker container `codex-asterinas-dev`
- Status: `EXR-WRITE-30` async supplement has landed, and the same-row checker has now produced `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/17_checker_serial.md`, `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/18_checker_serial_retry.md`, and `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/19_checker_serial_recheck.md`. The checker repaired two strictly local `inode.rs` issues, and the latest discriminator run now shows mixed evidence: the environment is unstable, but at least one instrumented carrier rerun reached the test body and then stalled after `before write_at`, so `W30-K3` still appears to have a surviving write-path hang once guest execution gets that far.

## Current Project State

- Current goal:
  - close `EXR-WRITE-30` with a same-row checker and then a final reviewer pass, while keeping `EXR-RESIZE-37` explicitly separate
- Current phase:
  - post-checker retry checkpoint with runtime proof still blocked
- Active or next component:
  - `EXR-WRITE-30`
  - the next lane is still `W30-K3`, but only as an environment-stable checker rerun rather than a new creator pass
- Latest accepted components:
  - `EXR-BOOT-34`
  - `EXR-CHARSET-32`
- Components in progress:
  - `EXR-WRITE-30` remains `SerialImplementing`
  - `EXR-NAMESPACE-29` and `EXR-SYNC-31` remain `Specified`
- Blocked components:
  - `EXR-RESIZE-37` is still planning-blocked on an explicit filesystem-owner release/reclaim decision
  - `W30-K3` is checker-blocked on unstable TCG/QEMU executable proof even after the latest in-scope `inode.rs` repairs

## What Just Landed

- Packet used:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-1921-creator-async-supplement-packet.md`
- Creator artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/16_creator_serial_repair.md`
- Production file touched:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- Landed shape:
  - the creator first added one owner-private inode-local `publication_gate` on `ExfatInode`
  - the checker then kept that same owner-local seam but repaired its concrete shape from `RwLock<()>` to `RwMutex<()>` after diagnosing that the original gate wrapped blocking I/O under a spin-based lock
  - `write_at()`, `read_at()`, and `PageCacheBackend::npages()` still route through that one inode-local publication seam
  - kept `resize` / truncate / deallocation ownership out of scope, consistent with `EXR-RESIZE-37`
- Commands run:
  - none; this remained a command-free creator lane

## Active Checker Lane

- Packet:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-2052-checker-async-supplement-packet.md`
- Target artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/17_checker_serial.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/18_checker_serial_retry.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/19_checker_serial_recheck.md`
- Lock state at dispatch:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/locks/checker-execution.lock/owner.toml` was absent before launch
- Checker scope:
  - re-run the three exact-name buffered-write regressions
  - prove the new `publication_gate` seam
  - keep resize/truncate semantics out of scope under `EXR-RESIZE-37`
- Checkpoint note:
  - `17_checker_serial.md` recorded the first same-row checker result: a local `npages()` self-deadlock repair plus one compact publication-focused ktest, but no clean executable pass.
  - The follow-on retry in `18_checker_serial_retry.md` diagnosed a second strictly local issue: the `publication_gate` was using spin-based `RwLock<()>` across blocking I/O and page-cache paths, so the checker repaired it to `RwMutex<()>`.
  - After that repair, the checker reran the exact-name minimal reproduction twice under the checker lock. Both reruns rebuilt and launched QEMU under TCG, but each stalled during early guest boot after `error: no suitable video mode found.` and never reached the ktest runner within the observed window.
  - The next recheck in `19_checker_serial_recheck.md` added two discriminators requested by the user:
    - an exact-name control ktest unrelated to `publication_gate`
    - temporary stage markers inside the lock-related carrier ktest
  - Result:
    - the control ktest also stalled during early guest boot within the observed window, so the environment is unstable outside `EXR-WRITE-30` too
    - however, the first instrumented carrier rerun emitted `before first read_at`, `after first read_at`, and `before write_at`, but never `after write_at`
    - that means at least one successful guest boot reached the carrier test body and then hung during `write_at`
  - The checker lock is now free again after those blocked reruns.

## Why This Matters

- The queued async supplement did not turn into a new owner or a filesystem-global coordinator.
- The repair stayed local to `ExfatInode` and preserved the existing call-local `ExfatInodeWriteState` pattern.
- The earlier main-agent concern about stale snapshot publication is now repaired at the owner-local boundary rather than deferred into `EXR-SYNC-31` or pulled back into `EXR-RESIZE-37`.

## Open Risks And Assumptions

- `EXR-WRITE-30` still needs a same-row checker pass to prove the new `publication_gate` seam and to ensure the earlier buffered-write regressions still pass.
- The current `publication_gate` seam is no longer using a spin-based lock across blocking work; the latest strictly local checker repair replaced it with `RwMutex<()>`.
- `03_designer_ktest.md` still contains historical resize scenarios from before the board split.
  - For `W30-K3`, treat the applicable obligations as buffered-write serialization and non-regression only.
  - Do not reopen resize/truncate checking under `EXR-WRITE-30`; that belongs to `EXR-RESIZE-37`.
- `EXR-WRITE-30`, `EXR-RESIZE-37`, `EXR-NAMESPACE-29`, `EXR-DIRECT-33`, and `EXR-INODE-META-36` still collide in `inode.rs`.
- The checker lock was free at dispatch time.
  - Re-check `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/locks/checker-execution.lock/owner.toml` before any second checker lane is considered.
- The latest blocker is no longer just a row-local logic bug.
  - The remaining gap is mixed: unstable TCG/QEMU runs can still stall before the test body, but the strongest positive signal now points at a surviving hang during `write_at` once the carrier ktest actually starts running.
  - The user-requested discriminators now support both halves of that picture:
    - a lock-unrelated control ktest also stalled during early boot
    - an instrumented carrier rerun reached `before write_at` and never emitted `after write_at`
  - Treat the next rerun as write-path debugging first, but do not assume the environment is healthy enough to reproduce the same guest progress every time.

## Recommended Next Actions

1. Keep `W30-K3` open as a checker rerun only: the next useful step is a fresh environment-stable exact-name rerun with narrower `write_at` tracing, not a new creator packet or another speculative owner split.
2. If a later rerun reaches the ktest runner and all four exact-name proofs pass, open `W30-R1` reviewer next; do not reopen `EXR-RESIZE-37` inside this row.
3. If a later rerun still stalls before the ktest runner, record it as an environment blocker and avoid opening `EXR-NAMESPACE-29` on the assumption that `inode.rs` is free for unrelated creator work.

## Resume Checklist

- Use `$exfat-main-agent`.
- Read `README.md`.
- Read `COMPONENT_INDEX.md`.
- Read this handoff after `copper-ledger-20260413-1958-boot34-accepted.md`.
- Treat `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/16_creator_serial_repair.md` as the authoritative record of the landed async supplement.
- Treat `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-2052-checker-async-supplement-packet.md` as the still-open checker packet.
- Read both `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/17_checker_serial.md` and `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/18_checker_serial_retry.md` before assuming the checker is merely queued.
- Also read `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/19_checker_serial_recheck.md` for the control-ktest plus `before write_at` evidence before reopening lock-focused debugging.
- Treat `EXR-RESIZE-37` as still separate and still missing its architect/designer closure.
- Inspect the checker lock before assuming `W30-K3` is idle or stale.

## Appendix: 2026-04-14 Async-Seam Note

- The larger 2026-04-14 helper inventory appendix was intentionally removed from this handoff.
  - That table overemphasized helper shape as the primary explanation for the current deadlock.
  - The stronger conclusion after the follow-up audit is that the present bug is better explained by an under-specified async seam across rows, not by a generic failure of helper factoring on its own.

### What Still Seems Correct

- `inode.rs::fill_cache_page_from_read_owner()` remains the clearest currently wrong local seam.
  - It is a page-cache backend helper that routes a cache-page fill back through the public buffered-read entrypoint `read_at()`.
- `io.rs::read_metadata_bytes()` remains a useful shared primitive, but its current name is misleading.
  - It is not a semantic metadata parser; it is the refactor-wide aligned block-slice read helper.
- The aligned read-modify-write pattern is still duplicated across `inode.rs`, `directory.rs`, `fat.rs`, and `bitmap.rs`.
  - That duplication is worth cleaning up later, but it does not look like the primary cause of the current self-deadlock.

### When Async Became A Whole-System Concern

- The first explicit async thinking entered on `2026-04-12`, not `2026-04-13`.
  - `EXR-READ-OPS-25/02_designer_async.md` defined buffered `read_at()` as a call-local read contract and explicitly pushed any stronger coordination into later page-cache work.
  - `EXR-PGCACHE-26/02_designer_async.md` then defined per-page cache-fill sequencing and explicitly kept any new lock hierarchy across inode, cache, or filesystem objects out of scope.
- The whole-system persistence and writeback picture became explicit on `2026-04-13`.
  - `EXR-WRITE-30/02_designer_async.md` named the downstream `write_page_async()` seam but still treated buffered-write serialization as inode-local and synchronous.
  - `EXR-SYNC-31/02_designer_async.md` then made `sync()`, `sync_all()`, `sync_data()`, and `write_page_async()` share one filesystem-owned serialization root instead of inventing separate owners.

### Current Main-Agent Reading

- The page-cache creator on `EXR-PGCACHE-26` did have a legitimate local design grab-handle:
  - reuse the already accepted buffered-read owner for page fills
  - keep the backend inode-local
  - avoid inventing a new global lock hierarchy because the async spec for that row explicitly kept that out of scope
- That local decision became fragile only after the later write-side publication seam was added on `2026-04-13`.
  - Once `write_at()` wrapped page-cache-affecting work inside one inode-local publication gate, the earlier page-fill path could recurse back into public inode I/O with no pre-agreed cross-row async contract for that recursion.
- So the best current explanation is:
  - this bug is not primarily “helper architecture drift”
  - it is a missing cross-row async/recursion design for the path `READ-OPS-25` -> `PGCACHE-26` -> `WRITE-30` -> future `SYNC-31`
  - the repair should therefore be planned as an async-seam correction, not just as a local stylistic helper cleanup
