<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff

## Metadata

- Fancy nickname: `amber-delta`
- Date: `2026-04-13 07:25 CST`
- Covered hours: user-requested continuity checks after `cinder-harbor`; targeted async audit for `EXR-PGCACHE-26`, designer repair for `EXR-WRITE-30`, and full creator-checker-reviewer closure for `EXR-DENTRY-WRITE-28`
- Author: `main-agent`
- Workspace: `/home/halifuda/asterinas`
- Container or environment: shared host workspace plus shared Docker container `codex-asterinas-dev`
- Status: `EXR-DENTRY-WRITE-28` is now accepted; `EXR-PGCACHE-26` remains accepted after a no-finding async audit; `EXR-WRITE-30` remains specified after a designer repair that pinned the write-state model; `EXR-NAMESPACE-29` is now fully unblocked behind the accepted directory-write row

## Environment Summary

- Checker execution is still serialized through `.agents/tools/checker_lock.sh`.
- Exact reruns still use `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <exact test>'`.
- `/dev/kvm` was present for the `EXR-DENTRY-WRITE-28` checker lane, but the guest run reported TCG warnings; the exact ktest proof still passed and is recorded in the checker artifact.

## Current Project State

- Accepted rows now include:
  - `EXR-DENTRY-WRITE-28`
  - `EXR-ALLOC-27`
  - `EXR-PGCACHE-26`
  - everything through `EXR-READ-OPS-25`
- Still specified:
  - `EXR-NAMESPACE-29`
  - `EXR-WRITE-30`
- Still planned:
  - `EXR-SYNC-31`
- Active runtime lanes:
  - none

## Recent Decisions

- Ran a command-free reviewer audit for `EXR-PGCACHE-26` async history.
  - Result: `32_reviewer_async_audit.md` reports no findings.
  - Decision: do not reopen `EXR-PGCACHE-26`; the absence of a separate concurrency patch loop is acceptable history for this row.
- Ran a command-free designer repair for `EXR-WRITE-30`.
  - Result: the designer set now names one explicit call-local `ExfatInodeWriteState`.
  - Decision: `EXR-WRITE-30` is still specified, but creator guesswork around mutation/publication shape is materially reduced.
- Drove `EXR-DENTRY-WRITE-28` through a full closure loop.
  - Creator landed `DirectoryEngine` write helpers in `directory.rs`.
  - Three narrow creator repairs followed:
    - restore logical directory offsets across chain-aware reads/writes,
    - continue growth from the earliest reusable tail instead of the old allocation end,
    - and preserve correct `Unused` termination semantics, including exact-EOF no-op handling.
  - Checker added and passed exact filtered regressions:
    - `directory_engine_reuses_deleted_slots_before_growth`
    - `directory_engine_preserves_location_when_rewrite_still_fits`
    - `directory_engine_consumes_committed_growth_for_directory_expansion`
  - Reviewer returned `30_reviewer_report.md` with no findings.
- Updated `COMPONENT_INDEX.md`.
  - `EXR-DENTRY-WRITE-28` is now `Accepted`.
  - `EXR-PGCACHE-26` now references the async audit artifact.
  - `EXR-WRITE-30` now records the 2026-04-13 designer repair summary.

## Open Risks And Assumptions

- `EXR-WRITE-30` is the sharpest remaining functionality gap because `write_at` and `resize` are still stubbed in production code.
- `EXR-NAMESPACE-29` is now cleaner to start because `EXR-DENTRY-WRITE-28` is accepted, but it will still collide in `inode.rs` and may need light `directory.rs` consumption changes.
- `EXR-SYNC-31` should stay planned until either `EXR-WRITE-30` or `EXR-NAMESPACE-29` creates real dirty producers that need explicit persistence ordering.

## Prior-Closure Caveat

- Closing `EXR-NAMESPACE-29`, `EXR-WRITE-30`, and `EXR-SYNC-31` would close the current owner-first board, but it would not automatically prove full prior closure.
- Remaining prior-visible gaps that still need either an owner row or an explicit non-goal decision:
  - backup boot-region fallback / compare policy
  - UTF-8 and NLS-backed name-conversion support
  - boot-flag persistence policy for `VolumeDirty`, `ClearToZero`, and possibly `PercentInUse`
  - Linux-style control surfaces not currently represented on the board, including volume-label mutation, FAT attribute ioctls, trim/discard, and forced shutdown
- Treat those items as evidence that a polished exFAT takeover still needs several follow-on modules or explicit non-goal closures after `EXR-SYNC-31`; do not assume `29/30/31` alone are sufficient for full-feature parity.
- Future main agents should treat these as continuity items instead of assuming the board already covers them implicitly.

## Recommended Next Actions

1. Default next creator frontier to `EXR-WRITE-30`.
2. Keep `EXR-NAMESPACE-29` immediately behind it unless product priorities now favor namespace work over file-data mutation.
3. Do not reopen `EXR-PGCACHE-26` for async history; use the new audit artifact as the continuity answer if the question comes up again.

## Resume Checklist

- Use `$exfat-main-agent`.
- Read `README.md`.
- Read `COMPONENT_INDEX.md`.
- Read this handoff after `cinder-harbor`.
- Treat `EXR-DENTRY-WRITE-28` as accepted; artifacts now run through:
  - `10_creator_serial.md`
  - `11_checker_serial.md`
  - `12_creator_serial_repair.md`
  - `14_creator_serial_repair.md`
  - `16_creator_serial_repair.md`
  - `30_reviewer_report.md`
- Treat `EXR-PGCACHE-26` as accepted and additionally audited by `32_reviewer_async_audit.md`.
- Treat `EXR-WRITE-30` as specified with the repaired designer set already in place.
- Treat `EXR-NAMESPACE-29` as specified and no longer blocked on directory-write acceptance.
