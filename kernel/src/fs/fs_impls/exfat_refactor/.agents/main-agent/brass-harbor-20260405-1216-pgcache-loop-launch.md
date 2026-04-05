<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff

## Metadata

- Fancy nickname: `brass-harbor`
- Date: 2026-04-05 12:55 CST
- Author: main-agent
- Covered hours: approximately `2.2` hours, from `2026-04-05 10:46 CST` to `2026-04-05 12:55 CST`
- Workspace: `/home/halifuda/asterinas`
- Container or environment: host workspace plus Docker container `codex-asterinas-dev`
- Status: consolidated handoff for this read-side wave cluster: `EXR-MOUNT-09`, delegated `EXR-READ-11A`, and delegated `EXR-PGCACHE-11B` are all accepted; `EXR-DIR-10` and `EXR-READ-11B` should enter the next creator round in parallel, `EXR-BITMAP-08B` should be the sidecar planning target, and `EXR-WRITE-13A` has been rolled back from `Specified` to `Architected`

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
  - last revalidated as `no-kvm`
- Validated commands:
  - `date '+%Y-%m-%d %H:%M %Z'`
  - `sed -n '1,220p' .agents/COMPONENT_INDEX.md`
- Known environment blockers:
  - shared worktree and shared command lane still apply
  - filtered test runs must continue to prove exact hit coverage
  - the user explicitly disallowed main-thread substitution for creator/checker/reviewer/final-checker work
  - QEMU runs continue under TCG-backed execution because `/dev/kvm` is unavailable

## Current Project State

- Current goal:
  - leave one compact resume point after this multi-wave stretch, with the read-side backend path accepted and the next loop reshaped around the user-approved maximum safe creator parallelism
- Current phase:
  - consolidated closeout complete
- Active or next component:
  - parallel creator round: `EXR-DIR-10` plus `EXR-READ-11B`
- Latest accepted components:
  - `EXR-MOUNT-09`
  - `EXR-READ-11A`
  - `EXR-PGCACHE-11B`
- Components in progress:
  - none
- Blocked components:
  - `EXR-WRITE-13A` should not be treated as creator-ready until `EXR-BITMAP-08B` is actually architected and specified

## Recent Decisions

- This consolidated note supersedes the separate per-wave handoffs I generated in this stretch:
  - `ember-causeway-20260405-1046-mount-loop-launch.md`
  - `sable-lattice-20260405-1128-read-loop-launch.md`
  - `iron-reticle-20260405-1148-read-redo-wave.md`
- `EXR-MOUNT-09` closed first and left two read-side successors specified in parallel:
  - `EXR-DIR-10`
  - `EXR-READ-11A`
- The protocol was tightened in this stretch so filtered `cargo osdk test` commands must prove hit coverage by exact suffix reasoning or explicit executed-test evidence; `exit 0` alone is not enough.
- The user then required strict delegation for critical-path loops. Because of that, the earlier local `EXR-READ-11A` acceptance was explicitly replaced by a fully delegated redo.
- The delegated `EXR-READ-11A` redo restored acceptance cleanly:
  - creator kept the mapping slice narrow and moved `map_logical_read_offset(...)` to consume `ExfatInodeReadView`
  - reviewer narrowed `walk_to_cluster_at_offset(...)` to return `(ClusterId, usize)` and removed the one-off chain accessor
  - checker and final-checker both reran the exact filtered ktests with source-backed suffix proof under the execution lock
- `EXR-PGCACHE-11B` was then run as the next sole creator component under the same delegated rule:
  - creator introduced the regular-file runtime plus the single `PageCacheBackend` bridge
  - first checker correctly stopped on a creator-owned return-type mismatch instead of patching around it
  - a bounded creator repair fixed only the `BioEnqueueError` to kernel `Error` conversion path
  - checker retry and final-checker reran the exact five filtered ktests under the execution lock and both passed
- Sidecar planning during these waves advanced:
  - `EXR-DIR-10` to `Specified`
  - `EXR-READ-11B` to `Specified`
  - `EXR-WRITE-13A` beyond what is now considered canonical
- The user overrode the earlier “next sole creator” conclusion for the next loop:
  - `EXR-DIR-10` and `EXR-READ-11B` should be launched in the same creator round
  - `mod.rs` linkage is treated as main-agent-owned integration work rather than as a creator write-set collision
  - `mod.rs` therefore should not be used as the bottleneck argument against that pairwise creator parallelism
- The user also called out that `EXR-WRITE-13A` was pushed too far before `EXR-BITMAP-08B` even had its own architected mutation boundary.
  - Because of that, `EXR-WRITE-13A` has been rolled back from `Specified` to `Architected`
  - its existing designer artifacts should be treated as provisional exploration rather than as canonical ready-to-implement spec
  - the next loop's planning sidecar should focus on `EXR-BITMAP-08B`

## Wave Record

- Scheduling or planning changes made in this wave:
  - collapsed this stretch into one surviving main-agent handoff
  - removed the need to keep multiple same-day resume notes for adjacent waves
- Components or passes advanced, accepted, repaired, blocked, or deferred in this wave:
  - `EXR-MOUNT-09` moved from `Specified` to `Accepted`
  - `EXR-DIR-10` moved from `Planned` to `Specified`
  - `EXR-READ-11A` moved from `Planned` to `Accepted`, then had its local acceptance superseded by a delegated redo, and ended this stretch accepted on subagent-owned evidence
  - `EXR-PGCACHE-11B` moved from `Planned` to `Specified`, then from `Specified` to `Accepted`
  - `EXR-READ-11B` moved from `Planned` to `Specified`
  - `EXR-WRITE-13A` moved from `Planned` to `Specified`, then was explicitly rolled back to `Architected`
- Protocol, template, or packet-shaping changes made in this wave:
  - filtered-test hit-proof requirements are now explicit in protocol and checker materials
  - command-free creator/reviewer packets were clarified to allow read-only inspection commands while still forbidding build, test, or runtime work
- Important facts intentionally removed from earlier drafts because they are no longer relevant:
  - per-wave scheduling detail that no longer changes the next resume decision

## Open Risks And Assumptions

- `EXR-PGCACHE-11B` is accepted, but its staging-only dead-code suppressions remain temporary until downstream runtime consumers arrive.
- `EXR-READ-11B` should consume the accepted page-cache backend and must not re-introduce a second backend or absorb mount/bootstrap work.
- `EXR-DIR-10` and `EXR-READ-11B` now share the next creator round by explicit user direction, so packetization should keep their creator write scopes disjoint and leave `mod.rs` linkage to the main agent.
- `EXR-BITMAP-08B` is now the most important planning gap on the write path.
- `EXR-WRITE-13A` should be reconsidered only after `EXR-BITMAP-08B` defines the bitmap-mutation boundary it depends on.

## Recommended Next Actions

1. Use this file as the only surviving handoff from this stretch.
2. Launch the next creator round with `EXR-DIR-10` and `EXR-READ-11B` in parallel.
3. Treat `mod.rs` linkage as main-agent-owned integration work, not as the reason to serialize those two creators.
4. Spend the same loop's planning sidecar on `EXR-BITMAP-08B`.

## Resume Checklist

- Read `README.md`.
- Read `PROTOCOL.md`.
- Read `COMPONENT_INDEX.md`.
- Read the latest main-agent handoff note.
- Verify the environment summary above still matches reality.
- Confirm this handoff still reflects strict delegated ownership before resuming the loop.
