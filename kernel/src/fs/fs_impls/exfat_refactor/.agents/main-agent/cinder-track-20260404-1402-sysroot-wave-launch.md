<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff

## Metadata

- Fancy nickname: `cinder-track`
- Date: 2026-04-04 14:02 CST
- Author: main-agent
- Workspace: `/home/halifuda/asterinas`
- Container or environment: host workspace plus Docker container `codex-asterinas-dev`
- Status: active wave record; test-surface cleanup and checker-lock script landed locally; `EXR-SYSROOT-06`, `EXR-UPCASE-07A`, `EXR-UPCASE-07B`, and `EXR-BITMAP-08A` are accepted, while `EXR-MOUNT-09` is the next dependency-ready big-loop component but remains unlaunched in this record

## Clean Summary

- Resume point:
  - this wave is cleanly paused after accepting `EXR-SYSROOT-06`, `EXR-UPCASE-07A`, `EXR-UPCASE-07B`, and `EXR-BITMAP-08A`
  - `EXR-MOUNT-09` is the next dependency-ready big-loop component
  - no new big loop has been launched yet
- Most important implementation outcomes:
  - `sysroot.rs` is accepted as the narrow root-system-entry scanner
  - `upcase_table.rs` is accepted as the loaded-table plus canonical fold-and-hash owner
  - `fileset.rs` now validates the canonical consumer path through `ExfatDentrySet::new(..., &ExfatUpcaseTable)`
  - `bitmap.rs` is accepted as the read-only allocation bitmap loader and occupancy surface
- Most important workflow outcomes:
  - `.agents/tools/checker_lock.sh` is the required checker lock entry point
  - main-thread takeover of unfinished command-free delegated work is forbidden
  - one main-agent loop may contain only one creator round, though that round may include multiple creators in parallel
- Post-loop review conclusions already settled:
  - accessor-only `ExfatUpcaseTable` field mirrors were removed
  - retained `fileset.rs` ktest staging helpers now carry explicit `TODO(EXR-UPCASE-07B)` exit conditions
  - the private free helpers in `sysroot.rs` and `bitmap.rs` are acceptable local shapes for now
  - no cleanup-worthy duplicate helper was found in the current touched-module set, including `upcase_table.rs:checksum32`

## Environment Summary

- Image or base environment:
  - host workspace
  - Docker container `codex-asterinas-dev`
- Working path:
  - host: `/home/halifuda/asterinas`
  - container: `/root/asterinas`
- Container name, if any:
  - `codex-asterinas-dev`
- KVM status:
  - revalidated as `no-kvm`
- Validated commands:
  - `docker ps --format '{{.Names}} {{.Status}}'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas && test -e /dev/kvm && echo kvm || echo no-kvm'`
  - `.agents/tools/checker_lock.sh status`
  - `.agents/tools/checker_lock.sh acquire ...`
  - `.agents/tools/checker_lock.sh release`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test fileset_valid_construction_round_trip_serialization'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test fileset_raw_name_aggregation'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test fileset_checksum_update_restores_validity'`
- Known environment blockers:
  - shared-worktree discipline still matters
  - unrelated production edits still exist in `boot_sector.rs` and `dentry.rs`
  - `cargo osdk test` currently runs under TCG and emits expected QEMU/TCG feature warnings

## Current Project State

- Current goal:
  - start the next executable big loop after the accepted loader and name-normalization wave while preserving the tightened packet/protocol rules
- Current phase:
  - the `EXR-UPCASE-07B` small repair loop is closed and accepted; the next big loop has not been started yet
- Active or next component:
  - `EXR-MOUNT-09` is next dependency-ready, but still paused pending explicit next-loop launch
- Latest accepted components:
  - `EXR-INOKEY-05A`
  - `EXR-INODE-05B`
- Components in progress:
  - none
- Blocked components:
  - `EXR-PGCACHE-11B` remains blocked on `EXR-READ-11A`

## Recent Decisions

- This wave uses a new standalone handoff instead of merging old notes forward. Older handoffs remain historical context; this file is the living record for the current wave only.
- The earlier `fileset.rs` test-surface cleanup was accepted locally:
  - test-only helpers used only by local `fileset` ktests were moved out of the production `impl`,
  - the cross-module `from_trusted_metadata` staging surface remains for now because `inode.rs` tests still depend on it.
- The testing/layout rule was tightened:
  - test-only helpers should live in local `mod tests` or a dedicated test-only support module by default,
  - `#[cfg(ktest)]` items inside production code now require an explicit cross-module reason and an exit plan.
- The checker execution lock is now concretized as `.agents/tools/checker_lock.sh` instead of ad hoc `mkdir` / `rm` sequences.
- `checker_lock.sh` was exercised both by self-test and by a real `cargo osdk test ...` path; it is the default lock entry point going forward.
- `EXR-SYSROOT-06` remains the next true dependency-ready component. No new parallel implementation sibling exists before its architect boundary stabilizes.
- `EXR-SYSROOT-06` architect artifact has now been accepted locally after a main-agent tightening pass:
  - the artifact now names concrete accepted dependencies,
  - it fixes the packet citation,
  - it requires a dedicated `sysroot.rs` implementation surface,
  - and it pins the output contract to typed immutable discovery facts for `UPCASE` and `BITMAP`.
- Explorer-side downstream analysis confirms the expected post-`SYSROOT` split:
  - `EXR-SYSROOT-06` should surface typed root-entry descriptors only,
  - `EXR-UPCASE-07A` and `EXR-BITMAP-08A` are truly parallel immediately after `SYSROOT`,
  - `EXR-UPCASE-07B` remains downstream only of `EXR-UPCASE-07A`.
- Based on that confirmation, the main agent has already archived and dispatched parallel architect packets for:
  - `EXR-UPCASE-07A`
  - `EXR-BITMAP-08A`
- Both downstream architect artifacts are now accepted locally:
  - `EXR-UPCASE-07A` is fixed as on-disk table loading and validation only, with fallback/name-layer policy deferred
  - `EXR-BITMAP-08A` is fixed as bitmap loading, validation, and read-only occupancy only, with allocation policy and mutation deferred
- `EXR-SYSROOT-06` designer artifacts are now accepted locally:
  - `01_designer_core.md`
  - `03_designer_ktest.md`
  - `02_designer_async.md` was intentionally omitted because the component is synchronous and read-only
- `EXR-SYSROOT-06` creator work has been dispatched as a command-free lane restricted to `sysroot.rs`, `mod.rs`, and the creator handoff.
- `EXR-SYSROOT-06` creator work has now landed locally:
  - new `sysroot.rs`
  - `mod.rs` wiring
  - creator handoff recorded
- Main-agent review found one bounded production defect and repaired it locally:
  - malformed chain-walk errors in `sysroot.rs` are now propagated instead of being silently normalized into end-of-directory.
- The `EXR-SYSROOT-06` checker packet is archived and dispatched, and it is required to use `.agents/tools/checker_lock.sh`.
- The first checker command suffix was too weak (`sysroot_`) because it could match zero tests; the checker evidence has been repaired to use `cargo osdk test sysroot::tests`.
- `EXR-SYSROOT-06` now has real serial-check evidence under TCG-backed QEMU and has been advanced into reviewer work.
- `EXR-SYSROOT-06` reviewer and final-checker closure are now complete:
  - reviewer found no blocking issues,
  - focused `cargo osdk test sysroot::tests` reran cleanly under the script-based checker lock,
  - the component is now accepted on the task board.
- After accepting both downstream architect artifacts, the main agent archived and dispatched:
  - `EXR-UPCASE-07A` designer work
  - `EXR-BITMAP-08A` designer work
- Both downstream designer sets are now accepted locally:
  - `EXR-UPCASE-07A` moved to `Specified`
  - `EXR-BITMAP-08A` moved to `Specified`
  - both intentionally omit `02_designer_async.md` because they stay synchronous and read-only
- To maximize parallelism without overlapping on `mod.rs`, the main agent approved one bounded creator deviation:
  - `EXR-UPCASE-07A` and `EXR-BITMAP-08A` creators each own only their new module file plus creator handoff,
  - `mod.rs` integration is reserved to the main agent after both creators land.
- Both downstream creator implementations are now present locally:
  - `upcase_table.rs` landed for `EXR-UPCASE-07A`,
  - `bitmap.rs` landed for `EXR-BITMAP-08A`,
  - `mod.rs` wiring for both modules was integrated centrally by the main agent after the creators returned.
- Current local review focus before checker dispatch:
  - verify the two staged loaders still match their narrow designer contracts,
  - then dispatch both serial checker lanes in parallel with focused `cargo osdk test ...` suffixes and script-based lock acquisition.
- Both checker lanes are now closed:
  - `EXR-BITMAP-08A` passed on the first focused `bitmap::tests` run,
  - `EXR-UPCASE-07A` needed one retry only because the first run was blocked by shared external compile failures in `bitmap.rs`, not because of an in-scope upcase defect.
- Both reviewer passes are now closed with no blocking findings.
- Both post-review final-check reruns are now clean:
  - `cargo osdk test bitmap::tests` exited `0` under TCG-backed QEMU,
  - `cargo osdk test upcase_table::tests` exited `0` under TCG-backed QEMU.
- `EXR-UPCASE-07A` and `EXR-BITMAP-08A` are now accepted on the task board.
- User process correction recorded on 2026-04-04 after wave closeout:
  - the main agent must not take over command-free delegated work just to preserve momentum; if a reviewer or other command-free subagent misreads the packet, the correct action is to redirect or repair that delegated lane rather than closing it and doing the work locally,
  - the main agent must not execute two creator-level components in one main-agent loop; creator work must stay bounded to the planned loop, while the next-wave architect/designer work can overlap in parallel instead.
- The scheduler protocol and workspace README were updated accordingly:
  - `PROTOCOL.md` now states these as positive scheduler rules,
  - `README.md` now summarizes the loop model in the workspace framing,
  - future loop planning should cite those rules directly instead of relying on handoff-only reminders.
- A draft architect packet for `EXR-UPCASE-07B` was archived locally, but it has not been dispatched because the user requested a planning pause for review before any next-wave execution.
- The user approved the revised loop model and asked that the next loop now proceed with:
  - one creator round maximum inside the loop,
  - the creator-bound checker/reviewer/final-check flow still running normally for that same implementation component,
  - a later dedicated reviewer wave to revisit the following code-quality questions instead of letting them block the current loop:
    - whether pure accessor helpers such as the current `ExfatUpcaseTable` field accessors should remain as standalone methods or be collapsed later,
    - whether module-local pure functions such as the current `bitmap.rs` helpers should stay as free functions or be re-homed as methods during a later quality pass.
- `EXR-UPCASE-07B` architect work is now accepted locally:
  - the architect artifact keeps `07B` strictly on the consumer side of `EXR-UPCASE-07A`,
  - it names the exact provisional `fileset.rs` raw-UTF-16 `name_hash` path that must be replaced,
  - and it keeps mount bootstrap, lookup orchestration, and namespace policy out of scope.
- Explorer-side codebase confirmation is now recorded for the designer wave:
  - current provisional `NameHash` in `fileset.rs` is the local `checksum_utf16(...)` over logical raw UTF-16 units,
  - the minimum downstream `07B` contract is one read-only service layered on `ExfatUpcaseTable` that hashes folded UTF-16 units without pulling in mount or lookup policy.
- `EXR-UPCASE-07B` designer artifacts are now accepted locally:
  - `01_designer_core.md`
  - `03_designer_ktest.md`
  - `02_designer_async.md` was intentionally omitted because the component stays synchronous and read-only.
- `EXR-MOUNT-09` architect work is now accepted locally:
  - mount ownership is fixed as bootstrap plus shared runtime state,
  - accepted `SYSROOT`, `UPCASE`, and `BITMAP` surfaces are consumed rather than rediscovered,
  - inode shaping, directory policy, page-cache behavior, and mutation remain explicitly outside mount.
- `EXR-MOUNT-09` designer artifacts are now accepted locally:
  - `01_designer_core.md`
  - `02_designer_async.md`
  - `03_designer_ktest.md`
  - the async artifact is justified here because mount publication has a real atomic shared-state handoff even though the constructor remains synchronous.
- `EXR-UPCASE-07B` creator and checker stages are now complete for this loop:
  - the creator added `ExfatUpcaseTable::name_hash()` and a staged fileset-side table-backed constructor path,
  - the checker added focused regressions in `upcase_table.rs` and `fileset.rs`,
  - focused `cargo osdk test upcase_table::tests` passed under TCG-backed QEMU,
  - but the checker found one blocking production defect: `ExfatDentrySet::validate()` still compares `name_hash` against raw `checksum_utf16(...)` instead of the canonical table-backed service.
- Because of the single-creator-round rule for this loop, the main agent stopped after that checker finding:
  - no repair creator was launched in the same loop,
  - reviewer and final-checker for `EXR-UPCASE-07B` were intentionally not started,
  - the next loop must decide whether to spend its single creator round on this repair.
- The next step is intentionally a small repair loop, not the next big wave:
  - one narrow `EXR-UPCASE-07B` creator retry is authorized only for the `fileset.rs` consumer-path defect from `11_checker_serial.md`,
  - `EXR-UPCASE-07B` reviewer is started in the same small loop as a report-only lane so it does not overlap the repair write set,
  - no `EXR-MOUNT-09` creator or any new big-loop component may start until this small loop closes.
- That small repair loop is now complete:
  - the creator lane was first corrected in-place rather than replaced, preserving the single creator round for the loop,
  - `fileset.rs` now makes `ExfatDentrySet::new(..., &ExfatUpcaseTable)` the canonical consumer boundary and confines structure-only validation to a ktest-only helper,
  - the retry checker and final-checker both reran focused `cargo osdk test fileset::tests` under the lock and passed under TCG,
  - the reviewer report's only blocking finding is now cleared by the repair plus retry evidence,
  - `EXR-UPCASE-07B` is accepted, but no next big loop was started afterward.
- Two code-quality questions are now explicitly queued for a later dedicated reviewer wave and must not block the current small repair loop:
  - whether accessor-only helpers on `ExfatUpcaseTable` are justified,
  - whether pure free functions in `sysroot.rs` and `bitmap.rs` should remain free functions or become methods.
- That follow-up reviewer wave has now been run before any next big loop launch:
  - `upcase_table.rs` reviewer removed the accessor-only `words()`, `byte_size()`, and `checksum()` surfaces because they had no non-test caller and only mirrored stored fields,
  - `fileset.rs` reviewer added explicit `TODO(EXR-UPCASE-07B)` exit-condition comments to the retained ktest-only staging helpers,
  - `sysroot.rs` reviewer concluded the local private free helpers are a coherent local shape for a narrow discovery-only scanner,
  - `bitmap.rs` reviewer concluded the local private free helpers are a coherent local shape for the read-only bitmap boundary,
  - no next big loop was started after this reviewer wave.

## Wave Record

- Scheduling or planning changes made in this wave:
  - reopened the main-agent wave record as a continuous editable handoff
  - selected `EXR-SYSROOT-06` as the active next-wave target under the current board and protocol
  - decided to overlap `EXR-SYSROOT-06` architecting with immediate downstream preplanning for the post-`SYSROOT` parallel wave
  - after architect acceptance, immediately launched the `EXR-SYSROOT-06` designer wave instead of waiting for later planning cleanup
  - used the accepted `SYSROOT` boundary to prelaunch `EXR-UPCASE-07A` and `EXR-BITMAP-08A` architect waves in parallel with `SYSROOT` design
  - after accepting the downstream architect artifacts, the active overlap is `SYSROOT` creator plus `UPCASE-07A` designer plus `BITMAP-08A` designer
  - after accepting the downstream designer artifacts, the active overlap is `SYSROOT` checker plus `UPCASE-07A` creator plus `BITMAP-08A` creator
  - after checker evidence repair, the active overlap is `SYSROOT` reviewer plus `UPCASE-07A` creator plus `BITMAP-08A` creator
  - after `SYSROOT` acceptance and downstream creator landing, the active overlap becomes `UPCASE-07A` checker plus `BITMAP-08A` checker
  - after `BITMAP` checker passed and `UPCASE` retry was launched, the active overlap became `BITMAP` reviewer plus `UPCASE` checker retry
  - after both reviewer artifacts landed, the active overlap became `BITMAP` final-checker plus `UPCASE` final-checker, serialized by the shared checker lock
  - after user approval of the revised loop model, the new loop starts with `UPCASE-07B` architect plus name-hash exploration, then advances to `UPCASE-07B` designer plus `MOUNT-09` architect
  - after accepting `UPCASE-07B` design and `MOUNT-09` architect, the loop now enters its single creator round with `UPCASE-07B` as the only implementation component
  - while the single `UPCASE-07B` creator/checker lane is active, `MOUNT-09` advances through architect and designer as the loop's command-free parallel work
  - after `UPCASE-07B` checker reported a blocking defect, the loop terminates without opening a second creator round
  - the follow-up small loop launches one `UPCASE-07B` creator retry plus one report-only `UPCASE-07B` reviewer lane; this is not a new big loop and does not reopen `MOUNT-09`
  - inside that small loop, the first repair attempt was redirected back into the same creator lane because it weakened validation instead of restoring the canonical boundary
  - after the corrected repair landed, the active work became `UPCASE-07B` checker retry and then `UPCASE-07B` final-checker, serialized through the shared checker lock
  - before any new big-loop launch, a dedicated parallel reviewer wave was run across today's touched modules: `upcase_table.rs`, `fileset.rs`, `sysroot.rs`, and `bitmap.rs`
- Components or passes advanced, accepted, repaired, blocked, or deferred in this wave:
  - `EXR-SYSROOT-06` moved from `Planned` to `Architected`
  - `EXR-SYSROOT-06` moved from `Architected` to `Specified`
  - the architect artifact was first delegated, then tightened locally by the main agent before acceptance
  - the `EXR-SYSROOT-06` designer packet was archived and dispatched
  - the `EXR-SYSROOT-06` creator packet was archived and dispatched
  - the `EXR-SYSROOT-06` checker packet was archived and dispatched
  - the `EXR-SYSROOT-06` reviewer packet was archived and dispatched
  - the `EXR-SYSROOT-06` final-checker packet was archived and dispatched
  - the `EXR-UPCASE-07A` and `EXR-BITMAP-08A` architect packets were archived and dispatched
  - the `EXR-UPCASE-07A` and `EXR-BITMAP-08A` designer packets were archived and dispatched
  - the `EXR-UPCASE-07A` and `EXR-BITMAP-08A` creator packets were archived and dispatched
  - the `EXR-UPCASE-07A` and `EXR-BITMAP-08A` checker packets were archived and dispatched
  - the `EXR-UPCASE-07A` checker retry packet was archived and dispatched
  - the `EXR-UPCASE-07A` and `EXR-BITMAP-08A` reviewer packets were archived
  - the `EXR-UPCASE-07A` and `EXR-BITMAP-08A` final-checker packets were archived and dispatched
  - the `EXR-UPCASE-07B` architect packet was archived and dispatched
  - the `EXR-UPCASE-07B` designer packet was archived and dispatched
  - the `EXR-MOUNT-09` architect packet was archived and dispatched
  - the `EXR-MOUNT-09` designer packet was archived and dispatched
  - the `EXR-UPCASE-07B` creator packet was archived and dispatched
  - the `EXR-UPCASE-07B` checker packet was archived and dispatched
  - the `EXR-UPCASE-07B` creator-retry packet was archived and dispatched
  - the `EXR-UPCASE-07B` reviewer packet was archived and dispatched
  - the `EXR-UPCASE-07B` checker-retry packet was archived and dispatched
  - the `EXR-UPCASE-07B` final-checker packet was archived and dispatched
  - the `EXR-UPCASE-07B` upcase-table follow-up reviewer packet was archived and dispatched
  - the `EXR-UPCASE-07B` fileset follow-up reviewer packet was archived and dispatched
  - the `EXR-SYSROOT-06` follow-up reviewer packet was archived and dispatched
  - the `EXR-BITMAP-08A` follow-up reviewer packet was archived and dispatched
  - `EXR-UPCASE-07A` moved from `Planned` to `Architected`
  - `EXR-BITMAP-08A` moved from `Planned` to `Architected`
  - `EXR-UPCASE-07A` moved from `Architected` to `Specified`
  - `EXR-BITMAP-08A` moved from `Architected` to `Specified`
  - `EXR-SYSROOT-06` moved from `Specified` to `Reviewing`
  - `EXR-SYSROOT-06` moved from `Reviewing` to `Accepted`
  - `EXR-UPCASE-07A` moved from `Specified` to `SerialImplementing`
  - `EXR-BITMAP-08A` moved from `Specified` to `SerialImplementing`
  - `EXR-UPCASE-07A` moved from `SerialImplementing` to `Accepted`
  - `EXR-BITMAP-08A` moved from `SerialImplementing` to `Accepted`
  - `EXR-UPCASE-07B` moved from `Planned` to `Architected`
  - `EXR-UPCASE-07B` moved from `Architected` to `Specified`
  - `EXR-UPCASE-07B` moved from `Specified` to `SerialChecked`
  - `EXR-UPCASE-07B` moved from `SerialChecked` to `Accepted`
  - `EXR-MOUNT-09` moved from `Planned` to `Architected`
  - `EXR-MOUNT-09` moved from `Architected` to `Specified`
  - `EXR-SYSROOT-06` directory and packet archive path have been created
- Protocol, template, or packet-shaping changes made in this wave:
  - added `.agents/tools/checker_lock.sh`
  - updated `README.md`, `PROTOCOL.md`, `TESTING_GUIDE.md`, `protocol/CHECKER.md`, and `protocol/TASK_PACKET_TEMPLATE.md` to point at the script-based lock flow
  - tightened test-helper placement guidance in `TESTING_GUIDE.md` and `ASTERINAS_CODE_QUALITY_PRIORS.md`
- Important facts intentionally removed from earlier drafts because they are no longer relevant:
  - none yet in this wave

## Open Risks And Assumptions

- `EXR-SYSROOT-06` must stay a root-directory system-entry scanner, not a mount object, not a general directory API, and not an early `UPCASE` or `BITMAP` loader.
- The accepted `SYSROOT` output contract is now sharper:
  - immutable discovery facts for root `UPCASE` and `BITMAP` entries,
  - including on-disk location and the raw entry fields later loaders need,
  - with duplicate or missing or malformed entry detection owned here rather than in later loaders.
- The existing legacy exFAT code still mixes root scanning, upcase loading, bitmap loading, and mount bootstrap. The architect packet must keep those concerns separated even when the source precedent intertwines them.
- `from_trusted_metadata` is still a temporary cross-module test surface. If later work removes that need, it should move into a dedicated test-only support module rather than drift back into production semantics.
- The checker-lock script now defines the concrete lock procedure. Future checker packets should cite the script command shape rather than re-describing raw directory operations.
- This handoff must be updated during the wave when architect dispatch, acceptance, or follow-on scheduling changes land. Do not wait until the end.
- Future main-agent loops should treat the following as hard scheduling constraints:
  - no main-thread takeover of unfinished command-free subagent work,
  - no second creator execution inside the same main-agent loop,
  - once one creator or checker lane is active, spend remaining parallel budget on architect/design preparation for the next dependency-ready wave instead of opening another creator lane.

## Recommended Next Actions

1. Open a fresh main-agent handoff in the new thread instead of continuing to append to this one.
2. Use `EXR-MOUNT-09` as the next big loop's sole implementation component:
   - launch `EXR-MOUNT-09` creator as that loop's only creator round,
   - then run its checker, reviewer, and final-checker in the normal bound flow,
   - do not start `DIR`, `READ`, or `BITMAP-08B` creator work in the same loop.
3. Spend the rest of that next loop's parallel budget on command-free preparation only:
   - preferred planning targets are `EXR-DIR-10` and `EXR-READ-11A`,
   - architect/design or explorer/packet-prep work is acceptable,
   - but any creator-level follow-on work must wait for a later loop after `EXR-MOUNT-09` is accepted.
4. In that next thread, treat the following as already-settled and do not reopen them unless a new component reintroduces them:
   - `ExfatUpcaseTable` accessor cleanup is done,
   - `sysroot.rs` and `bitmap.rs` free-helper shape is accepted,
   - current duplicate-helper review found no cleanup-worthy cross-module duplicate.
5. Keep using this handoff as historical context only; do not merge it forward.

## Resume Checklist

- Read `README.md`.
- Read `PROTOCOL.md`.
- Read `COMPONENT_INDEX.md`.
- Read the latest main-agent handoff note.
- Verify the environment summary above still matches reality.
- Confirm this handoff already reflects the material implementation and protocol changes from this wave before committing or handing off.
