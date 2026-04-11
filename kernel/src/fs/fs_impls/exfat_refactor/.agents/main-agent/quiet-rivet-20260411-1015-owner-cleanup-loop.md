<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff

## Metadata

- Fancy nickname: `quiet-rivet`
- Date: 2026-04-11 10:15 CST
- Covered hours: narrow 2026-04-11 maintenance loop for owner-first cleanup after the orphan audit
- Author: main-agent
- Workspace: `/home/halifuda/asterinas`
- Container or environment: shared host workspace plus shared Docker container `codex-asterinas-dev`
- Status: `EXR-FS-OPEN-22` remains accepted and now also has a passing owner-cleanup creator/checker loop for the `read_chain_bytes` orphan-helper relocation

## Historical Continuity

- Resume baseline before this loop: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/harbor-signal-20260410-1120-checker-loop-18-19.md`
- This loop first ran a transient orphan-helper review across the current Rust modules, excluding `test_support.rs`.
- That transient review concluded that only `fs.rs::read_chain_bytes` justified immediate owner-first cleanup; the other currently visible helper or standalone-type seams were left alone.
- Per later user instruction, the transient audit reports, packets, and maintenance step artifacts from this loop were removed. This handoff is the only persistent record of that work.

## Environment Summary

- Shared checker lane is still serialized through `.agents/tools/checker_lock.sh`.
- The validated command shape remains:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <suffix>'`
- This loop's checker evidence was TCG-backed, not KVM-proven.

## Current Project State

- `EXR-FS-OPEN-22` remains accepted.
- A narrow post-accept owner-cleanup loop landed in production code:
  - `read_chain_bytes` was moved from a module-scope free helper into a private `ExfatFs` owner method in `fs.rs`
  - the upcase prerequisite path now calls that owner-local method
- A targeted checker rerun passed on the exact ktest suffix `fs::tests::root_mount_sequence_installs_prerequisites_before_publishing_root`.
- No reviewer was opened for this cleanup because the change was a narrow owner-private relocation with a clean targeted checker result and no widened behavior.

## Active Work Slice Matrix

There is no active writer after this maintenance loop.

| Slice ID | Parent Unit | Goal | Write Set | Depends On | May Overlap With | Lane Class | Status | Source Artifact | Packet |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-FS-OPEN-22-OWNER-CLEANUP-20260411` | `EXR-FS-OPEN-22` | Move `read_chain_bytes` from a module-scope free helper into an `ExfatFs` owner-private method without changing mount/open behavior | `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` | accepted `EXR-FS-OPEN-22`, transient orphan assessment | command-free lanes outside checker execution | creator then serialized checker | passed | transient orphan assessment in this loop | transient creator/checker packets in this loop |

## Recent Decisions

- Excluded `test_support.rs` from production orphan cleanup.
- Ran a reviewer-only assessment on the remaining orphan candidates and accepted its conclusion that only `read_chain_bytes` justified immediate cleanup.
- Treated this as a narrow maintenance loop inside `EXR-FS-OPEN-22`, not as a new board row.
- Opened a command-free creator pass to re-land `read_chain_bytes` under `ExfatFs`.
- Opened a targeted checker pass because the cleanup still touched production code and had not been compile-verified by the creator.
- Accepted the checker proof from the exact suffix `fs::tests::root_mount_sequence_installs_prerequisites_before_publishing_root`; no broader checker sweep was needed.
- Tightened reviewer guidance in both the repo-local reviewer protocol and the reusable subagent skill so future reviewer lanes must inspect owner-first landing form more explicitly, including free helpers, standalone structs or enums, emitted record shapes, temporary seams, and whether a borderline surface is acceptable-for-now, document-and-defer, or refactor-now.
- Removed the transient audit directory, task packets, and maintenance step artifacts afterward on user request, keeping only this handoff as persistent continuity.

## Wave Record

- Scheduling or planning changes made in this wave:
  - commissioned a transient orphan-helper review grouped by module order
  - commissioned a follow-on assessment excluding `test_support.rs`
  - adopted the assessment conclusion that only `read_chain_bytes` needed immediate owner-first cleanup
  - launched a narrow creator/checker maintenance loop under `EXR-FS-OPEN-22`
  - updated the reviewer protocol and reviewer skill reference to require more explicit owner-first seam checks in future review passes
  - removed the transient audit and maintenance artifacts afterward, keeping only this handoff
- Components or passes advanced, accepted, repaired, blocked, or deferred in this wave:
  - `EXR-FS-OPEN-22` owner-cleanup creator pass landed successfully
  - `EXR-FS-OPEN-22` owner-cleanup checker passed on the targeted mount-sequence ktest
  - all other current orphan candidates were deferred as acceptable-for-now or document-and-defer

## Open Risks And Assumptions

- The remaining orphan-audit candidates are still only assessed, not rewritten.
- If future owner-first cleanup resumes, the next likely non-`test_support` candidate to document more clearly is `boot_sector.rs::persistent_volume_flags`, but it was not judged urgent enough for immediate refactoring.
- Because the transient audit artifacts were removed, any future orphan-helper cleanup will need to re-establish evidence locally instead of reopening deleted reports.

## Recommended Next Actions

1. Resume normal read-side planning from the accepted `EXR-DIR-OPS-23` and architected `EXR-FILE-MAP-24` baseline.
2. If owner-first cleanup budget appears again, re-establish the evidence locally under the tightened reviewer rules instead of assuming the deleted transient audit artifacts still exist.

## Resume Checklist

- Use `$exfat-main-agent`.
- Read `README.md`.
- Read `COMPONENT_INDEX.md`.
- Read this note after `harbor-signal` if the next task depends on the orphan-helper maintenance loop.
- Treat `EXR-FS-OPEN-22` as still accepted, with the additional maintenance fact that `read_chain_bytes` now lands as an owner-private `ExfatFs` method and the targeted root-mount-sequence ktest passed in this loop.
