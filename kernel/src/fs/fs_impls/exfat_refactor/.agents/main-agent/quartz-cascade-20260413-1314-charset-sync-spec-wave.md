<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff

## Metadata

- Fancy nickname: `quartz-cascade`
- Date: `2026-04-13 13:14 CST`
- Covered hours: user-requested parallel architect/designer wave for `EXR-CHARSET-32` and `EXR-SYNC-31`, the post-charset designer repair for `EXR-NAMESPACE-29`, then the `EXR-CHARSET-32` checker/reviewer loop, `EXR-BOOT-34` architect plus designer kickoff, and a same-day `EXR-CHARSET-32` checker retry after tightening the filtered-ktest protocol
- Author: `main-agent`
- Workspace: `/home/halifuda/asterinas`
- Container or environment: shared host workspace plus shared Docker container `codex-asterinas-dev`
- Status: `EXR-CHARSET-32` is now `Accepted` after the retry checker completed exact-name proof with no additional production edits; `EXR-SYNC-31` remains `Specified`; `EXR-NAMESPACE-29` is back to `Specified` after a same-wave designer repair that consumes the new charset boundary; `EXR-WRITE-30` remains `SerialImplementing`, buffered-only, and now has both a landed creator repair for non-empty-chain growth plus an active narrow checker lane for the current `write_at` / growth slice; `EXR-BOOT-34` is now `Specified` after an architect-plus-designer wave that keeps the row above boot parsing, below mount/open orchestration, and separate from sync ordering

## Environment Summary

- Checker execution is still serialized through `.agents/tools/checker_lock.sh`.
- One checker lane was opened and completed in this wave for `EXR-CHARSET-32`, then a retry checker lane reran the proof after the unrelated shared-build compile failure at `directory.rs:794` was fixed.
- Filtered `cargo osdk test` proof should now use only complete test names or complete, source-justified path suffixes after a same-day protocol/skill clarification that prefix fragments such as `charset_` are not trustworthy proof.
- `EXR-WRITE-30` now also has an active checker lane holding the execution lock from `2026-04-13 18:23:34 CST`.
  - The lock metadata currently names these exact commands:
    - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
    - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_carrier_snapshots_metadata_and_exercises_buffered_read'`
    - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_buffered_write_grows_an_empty_file_and_publishes_allocation_facts'`
    - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_buffered_write_extends_a_non_empty_file_across_growth'`
  - Latest observed guest output comes from `/home/halifuda/asterinas/qemu-serial.log`; no checker artifact has been written yet.

## Current Project State

- Accepted rows still include everything through `EXR-DENTRY-WRITE-28`.
- Specified rows now include:
  - `EXR-NAMESPACE-29`
  - `EXR-SYNC-31`
  - `EXR-BOOT-34`
- Serial implementing rows now include:
  - `EXR-WRITE-30`
- Accepted rows now also include:
  - `EXR-CHARSET-32`
- Planned rows remain:
  - `EXR-DIRECT-33`
  - `EXR-VOLLABEL-35`
  - `EXR-INODE-META-36`
- Active runtime lanes:
  - `K3`: `EXR-WRITE-30` checker is active from packet `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-1821-checker-serial-packet.md`

## Active Work-Slice Matrix

| Lane | Row | Stage | Status | Notes |
| --- | --- | --- | --- | --- |
| `C1` | `EXR-WRITE-30` | creator repair | completed | The first slice landed `write_at`, the call-local `ExfatInodeWriteState`, gap zero-fill, and empty-file growth, and the same-row repair then stitched newly allocated clusters onto already non-empty file chains by preserving contiguity only when possible and otherwise materializing a combined FAT-backed chain inside `inode.rs`. |
| `K3` | `EXR-WRITE-30` | checker | active | This narrow checker validates the currently landed `write_at` / growth slice with exact test names only. It does not pretend `resize` or truncate are done; those remain deferred follow-on creator slices even if the checker passes. |
| `K2` | `EXR-CHARSET-32` | checker retry | completed | Creator landed, the first checker added the required local charset regressions, reviewer found no new issues, and the retry packet reran proof with complete test names only after the unrelated `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs:794` blocker was fixed. The retry completed exact-name proof and moved the row to `Accepted`. |
| `D5` | `EXR-BOOT-34` | designer | completed | The command-free designer pass from `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BOOT-34/20260413-1421-designer-packet.md` landed the core, async, and ktest artifacts and returned the row to `Specified`. |
| `C3` | `EXR-NAMESPACE-29` | creator | contingent | Designer set is repaired and architect ownership still holds, but creator work must consume the landed `EXR-CHARSET-32` boundary instead of reparsing raw `&str`. |
| `C4` | `EXR-SYNC-31` | creator | deferred | Design is now stable, but the row should stay behind actual dirty-producer landings and remain flush-ordering only. |

## Recent Decisions

- Archived architect packets for:
  - `EXR-CHARSET-32` at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-CHARSET-32/20260413-1301-architect-packet.md`
  - `EXR-SYNC-31` at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-SYNC-31/20260413-1301-architect-packet.md`
- Accepted architect artifacts for:
  - `EXR-CHARSET-32` at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/00_architect.md`
  - `EXR-SYNC-31` at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYNC-31/00_architect.md`
- Archived designer packets for:
  - `EXR-CHARSET-32` at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-CHARSET-32/20260413-1306-designer-packet.md`
  - `EXR-SYNC-31` at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-SYNC-31/20260413-1304-designer-packet.md`
  - `EXR-NAMESPACE-29` repair at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-NAMESPACE-29/20260413-1307-designer-repair-packet.md`
- Archived and dispatched a creator packet for:
  - `EXR-CHARSET-32` at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-CHARSET-32/20260413-1403-creator-serial-packet.md`
  - Narrow write set: `fs.rs`, `inode.rs`, and `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/10_creator_serial.md`
  - Explicit creator boundary: land the `ExfatFs` charset owner plus the accepted read-side consumer migration; do not touch namespace mutation, low-level file-record constructors, or any checker artifacts
- Pre-staged a checker packet for:
  - `EXR-CHARSET-32` at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-CHARSET-32/20260413-1409-checker-serial-packet.md`
  - Planned checker proof: new local `fs.rs` charset tests plus exact reruns of the accepted `inode.rs` lookup/readdir regressions under the checker lock
- Dispatched the `EXR-CHARSET-32` checker lane from that packet.
- Recorded the completed `EXR-CHARSET-32` checker result at:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/11_checker_serial.md`
  - Outcome: checker-owned `charset_` regressions landed, but required filtered proof is blocked by unrelated compile failure at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs:794`.
- Archived and dispatched a reviewer packet for:
  - `EXR-CHARSET-32` at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-CHARSET-32/20260413-1418-reviewer-packet.md`
- Recorded the completed `EXR-CHARSET-32` reviewer result at:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/30_reviewer_report.md`
  - Outcome: no new reviewer findings in `fs.rs` or the narrow `inode.rs` migration; only the already-known checker blocker remains.
- Archived and dispatched a retry checker packet for:
  - `EXR-CHARSET-32` at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-CHARSET-32/20260413-1744-checker-retry-packet.md`
  - Scope: rerun the blocked checker proof using only complete test names for the five `charset_*` regressions and the four accepted `inode.rs` lookup/readdir regressions, writing the retry report to `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/12_checker_serial_retry.md`
- Recorded the completed `EXR-CHARSET-32` retry checker result at:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/12_checker_serial_retry.md`
  - Outcome: the five local charset regressions and the four accepted lookup/readdir regressions all passed under the checker lock using complete test names, so the row can leave `Blocked` and move to `Accepted` without further production edits.
- Archived and dispatched a creator packet for:
  - `EXR-WRITE-30` at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-1755-creator-serial-packet.md`
  - Scope: first buffered-write slice only, limited to `write_at`, one call-local `ExfatInodeWriteState`, and just the growth logic needed for extending writes; do not widen into `resize`, `O_DIRECT`, or sync-ordering work.
- Recorded the completed first `EXR-WRITE-30` creator result at:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/10_creator_serial.md`
  - Outcome: `write_at` now owns buffered writes, valid-size gap zero-fill, and empty-file growth on `ExfatInode`, but the creator also identified one remaining same-row gap before checker: extending writes on already non-empty files still need the newly allocated clusters stitched onto the preexisting chain.
- Archived and dispatched a creator repair packet for:
  - `EXR-WRITE-30` at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-1807-creator-repair-packet.md`
  - Scope: repair non-empty-file growth reachability only by stitching newly allocated clusters onto the existing file chain inside `inode.rs`; do not widen into `resize`, truncate, direct I/O, sync ordering, or new public allocation helpers.
- Recorded the completed `EXR-WRITE-30` creator repair result at:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/12_creator_serial_repair.md`
  - Outcome: extending writes on already non-empty files now either preserve contiguity when the appended committed run is immediately adjacent or materialize a combined FAT-backed chain owner-privately in `inode.rs`; a same-pass static audit also caught and fixed two local `state.metadata.size` field-reference typos before checker dispatch.
- Archived and dispatched a checker packet for:
  - `EXR-WRITE-30` at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-1821-checker-serial-packet.md`
  - Scope: exact-name proof for the landed `write_at` / growth slice only, using `inode_carrier_snapshots_metadata_and_exercises_buffered_read`, `inode_buffered_write_grows_an_empty_file_and_publishes_allocation_facts`, and `inode_buffered_write_extends_a_non_empty_file_across_growth`; `resize` remains explicitly unverified in this packet because it is still a deferred slice.
- Observed the first active `EXR-WRITE-30` checker failure through `/home/halifuda/asterinas/qemu-serial.log` before the checker subagent returned:
  - `inode_carrier_snapshots_metadata_and_exercises_buffered_read` failed first under QEMU.
  - The log currently shows `/root/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs:1868:9` with `assertion left == right failed`, `left: [0, 0, 0, 0]`, `right: [161, 178, 195, 212]`.
  - No checker artifact exists yet at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/13_checker_serial.md`, so the failure is only captured in the serial log and this handoff so far.
- Archived and dispatched an architect packet for:
  - `EXR-BOOT-34` at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BOOT-34/20260413-1413-architect-packet.md`
  - Scope: define `ExfatFs` ownership for backup-boot fallback / compare policy and persistent `VolumeDirty` / `ClearToZero` / `PercentInUse` policy without widening into sync or mount-open cloning
- Accepted the `EXR-BOOT-34` architect artifact at:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/00_architect.md`
- Archived and dispatched a designer packet for:
  - `EXR-BOOT-34` at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BOOT-34/20260413-1421-designer-packet.md`
  - Scope: specify the owner-private boot fallback result shape plus persistent boot-flag intent and the `EXR-FS-OPEN-22` / `EXR-SYNC-31` handoffs without reopening boot parsing or sync ownership
- Accepted the `EXR-BOOT-34` designer set at:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/01_designer_core.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/02_designer_async.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/03_designer_ktest.md`
  - Outcome: `VolumeDirty` and `ClearToZero` are now pinned as owner-private boot-region outputs, `PercentInUse` stays observational by default, `EXR-FS-OPEN-22` consumes only the trusted boot source, and `EXR-SYNC-31` consumes only the published dirty-boot intent.
- Accepted designer sets for:
  - `EXR-CHARSET-32`, which now fixes the output contract to validated UTF-16 text plus length under `ExfatFs`, with `EXR-UPCASE-20` still the sole fold/hash owner
  - `EXR-SYNC-31`, which now fixes `ExfatFs` as the single filesystem-wide flush-ordering root for `FileSystem::sync()`, inode `sync_all()` / `sync_data()`, and `write_page_async()`
- Chose not to reopen `EXR-NAMESPACE-29` architect work.
  - Reason: the architect owner boundary for namespace mutation on `ExfatInode` still holds after the tail reshape.
  - Repair needed only at designer level once `EXR-CHARSET-32` made the conversion boundary explicit.
- Updated `.agents/COMPONENT_INDEX.md`.
  - `EXR-CHARSET-32` -> `Accepted`
  - `EXR-SYNC-31` -> `Specified`
  - `EXR-NAMESPACE-29` -> `Specified`
  - `EXR-WRITE-30` -> `SerialImplementing`
  - `EXR-BOOT-34` -> `Specified`
  - the `EXR-CHARSET-32` row now includes creator, checker, reviewer, and retry-checker artifacts with exact-name proof completed
  - the `EXR-WRITE-30` row now records the first active creator slice on the buffered-write path
- Updated protocol / skill guidance for filtered ktests:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/PROTOCOL.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CHECKER.md`
  - `/mnt/c/Users/anyud/.codex/skills/exfat-main-agent/SKILL.md`
  - `/mnt/c/Users/anyud/.codex/skills/exfat-subagent-workflow/SKILL.md`
  - Decision: prefix fragments such as `charset_` are not trustworthy `cargo osdk test` proof; packets and checker execution should use complete test names or complete, source-justified path suffixes.

## Wave Record

- `EXR-CHARSET-32` architect result:
  - `ExfatFs` owns the only stable VFS-visible name / label conversion boundary.
  - Linux byte-string and optional NLS policy are explicit non-goals for this row; the stable external contract is Asterinas `&str`.
- `EXR-CHARSET-32` designer result:
  - the produced value is validated UTF-16 text plus length only
  - no fold/hash state attaches here
  - `EXR-NAMESPACE-29` and `EXR-VOLLABEL-35` must consume converted values instead of reparsing raw text
  - accepted read-side `lookup` / `readdir_at` consumers are now also part of the creator migration plan because current `inode.rs` still carries local `encode_utf16()` / `String::from_utf16()` calls
  - low-level trusted UTF-16 constructors such as `ExfatDentrySet::from_trusted_metadata(..., raw_name_units, ...)` may remain as leaf seams, but creator work should ensure business-facing name paths no longer feed them ad hoc UTF-16 outside the charset owner
- `EXR-SYNC-31` architect/designer result:
  - `ExfatFs` is the only filesystem-wide persistence owner
  - `sync_all()` / `sync_data()` are thin inode delegates into the same owner-private root
  - `write_page_async()` stays downstream to that same flush-ordering boundary
- `EXR-NAMESPACE-29` designer repair result:
  - namespace preflight now explicitly consumes the validated converted-name from `EXR-CHARSET-32`
  - only after that handoff does `EXR-UPCASE-20` receive UTF-16 units for fold/hash work
  - raw `&str` parsing stays out of `inode.rs`
- `EXR-CHARSET-32` serial checker/reviewer result:
  - checker landed five `charset_` regressions in `fs.rs`
  - reviewer found no new owner-boundary or migration issues in `fs.rs` or the narrow `inode.rs` repair
  - the retry checker later reran all nine targeted regressions with complete test names after the unrelated `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs:794` blocker was fixed, and that exact-name proof completed without further production edits
- `directory.rs` / `VmIo` continuity result:
  - the current `directory.rs` `VmIo` import is not a new accidental extra; `write_bytes` on `dyn BlockDevice` depends on the `VmIo` trait in scope
  - the earlier `EXR-DENTRY-WRITE-28` checker already recorded a local `directory.rs` fix that imported `VmIo` for the write path
  - the most likely explanation for the same error resurfacing later is continuity drift in the shared worktree rather than a brand-new checker discovery
- `EXR-WRITE-30` creator kickoff result:
  - the row opened its first serial creator lane only after `EXR-CHARSET-32` finished retry-checker proof
  - the first slice landed `write_at`, one call-local `ExfatInodeWriteState`, valid-size gap zero-fill, and empty-file growth while keeping `resize`, `O_DIRECT`, sync ordering, and inode-admin control out of scope
  - the same creator result also exposed one remaining creator-level gap before checker: when growth allocates new clusters for an already non-empty file, the new chain must still be stitched onto the old chain coherently
  - a follow-on same-row creator repair then landed that gap by preserving contiguous growth only when the appended run is immediately adjacent and otherwise materializing a FAT-backed combined chain with owner-private helpers in `inode.rs`
  - a same-wave static audit caught and locally fixed two `state.metadata.size` compile-time typos before checker dispatch
  - the follow-on narrow checker is now active, but the first exact-name test already fails in guest output before any checker artifact has been emitted, so the row is not ready to advance
- `EXR-BOOT-34` architect result:
  - `ExfatFs` owns boot-source selection above validated primary boot facts plus persistent `VolumeDirty` / `ClearToZero` intent
  - `PercentInUse` remains bounded to policy interpretation only and may stay observational if the design does not need it
  - boot parsing stays in `EXR-BOOT-01`, mount/open sequencing stays in `EXR-FS-OPEN-22`, and flush ordering stays in `EXR-SYNC-31`
- `EXR-BOOT-34` designer result:
  - creator work is now fixed to a `fs.rs`-centered owner-private policy surface rather than a generic recovery shell
  - `VolumeDirty` and `ClearToZero` are published together as persistent boot intent
  - `PercentInUse` stays observational and cannot silently become a free-space owner
  - checker obligations are explicit for source selection stability, dirty-intent publication, `ClearToZero` pre-mutation behavior, and sync-side consumption of already-published boot intent

## Open Risks And Assumptions

- `EXR-WRITE-30` remains the sharpest production functionality gap because `resize` is still stubbed even though the current `write_at` / growth slice is now in checker.
- `EXR-WRITE-30` currently has an active checker lock and an as-yet-unwritten checker artifact. If this thread is resumed after interruption, the next main-agent must first decide whether the lock is still genuinely active or has become stale before starting any new checker lane.
- The latest visible failure for `EXR-WRITE-30` is not a generic harness timeout: `inode_carrier_snapshots_metadata_and_exercises_buffered_read` read back zeros instead of `[0xA1, 0xB2, 0xC3, 0xD4]` at the expected offset.
- `EXR-NAMESPACE-29` is no longer board-blocked, but its creator lane should stay behind the active `inode.rs` write lane unless a future packet proves the file overlap is gone.
- `EXR-CHARSET-32` is not an `fs.rs`-only row anymore in practice; successor rows must respect the landed narrow `inode.rs` consumer migration and avoid reintroducing local text-conversion policy there.
- `EXR-CHARSET-32` is accepted now, but successor rows should preserve the exact-name checker discipline because prefix fragments can still silently yield `0 passed; 0 failed`.
- `EXR-SYNC-31` is now design-stable, but creator work should remain behind real dirty producers and must not be used as a bucket for boot policy, label control, direct I/O, or inode-admin cleanup.
- `EXR-CHARSET-32`, `EXR-SYNC-31`, `EXR-BOOT-34`, and `EXR-VOLLABEL-35` all collide in `fs.rs`; keep creator waves serialized there.
- `EXR-NAMESPACE-29`, `EXR-WRITE-30`, `EXR-DIRECT-33`, and `EXR-INODE-META-36` all collide in `inode.rs`; do not treat them as file-parallel creator lanes.
- `EXR-DIRECT-33` remains the explicit owner for `O_DIRECT`; do not let later `EXR-WRITE-30` packets smuggle direct-I/O policy back into buffered write work.

## Recommended Next Actions

1. First resolve the currently active `EXR-WRITE-30` checker lane safely:
   - if the checker subagent is still active, wait for it to emit `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/13_checker_serial.md` and release the lock;
   - if the subagent is gone but the lock remains, perform a main-agent stale-lock review before any new checker work.
2. Once the checker lane is safely closed, treat the first visible failure as the next immediate debug target: `inode_carrier_snapshots_metadata_and_exercises_buffered_read` currently reads back zeros where the write test expected `[0xA1, 0xB2, 0xC3, 0xD4]`.
3. Keep `EXR-NAMESPACE-29` behind the active `inode.rs` write lane even though the charset dependency is now accepted.
4. Keep `EXR-SYNC-31` in the queue, but only open its creator lane after at least one dirty-producer landing clarifies the real flush path it must consume.
5. Treat `EXR-BOOT-34` as creator-ready on paper, but do not open its creator lane in parallel with other `fs.rs` creator work.
6. Preserve the exact-name filtered-ktest rule in future checker packets and proofs.

## Resume Checklist

- Use `$exfat-main-agent`.
- Read `README.md`.
- Read `COMPONENT_INDEX.md`.
- Read this handoff after `topaz-bridge`.
- Treat `EXR-CHARSET-32` and `EXR-SYNC-31` as fully architected and designed.
- Treat `EXR-CHARSET-32` as accepted with creator/checker/reviewer artifacts plus the completed retry checker report at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/12_checker_serial_retry.md`.
- Treat `EXR-NAMESPACE-29` as repaired at designer level without needing an architect rewrite.
- Treat `EXR-WRITE-30` as still `SerialImplementing`: the first creator artifact landed at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/10_creator_serial.md`, the creator repair landed at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/12_creator_serial_repair.md`, and the active narrow checker lane now runs from `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-1821-checker-serial-packet.md`.
- Before starting any new checker work, inspect `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/locks/checker-execution.lock/owner.toml` and `/home/halifuda/asterinas/qemu-serial.log`.
- If `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/13_checker_serial.md` still does not exist, use the serial log failure at `inode.rs:1868:9` as the authoritative latest evidence for the active checker lane.
- Treat `EXR-BOOT-34` as specified with artifacts through `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/03_designer_ktest.md`.
- Keep `EXR-DIRECT-33`, `EXR-VOLLABEL-35`, and `EXR-INODE-META-36` on-board as explicit planned rows.
