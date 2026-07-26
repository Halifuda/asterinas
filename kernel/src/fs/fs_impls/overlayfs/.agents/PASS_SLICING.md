<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Pass Slicing Ledger

This file is the durable main-agent-owned record of how meso-level Architect / Designer contracts are split into pass-level Creator, Checker, and Reviewer work.

`SYSTEM_BLUEPRINT.md` remains the active status board. This ledger records the scheduling decision, covered-micro boundary, and rationale so later main agents do not rediscover or accidentally widen previous pass slices.

## Rules

- Only the main agent updates this file.
- Record a decision before or at the same time a Creator, Checker, or Reviewer packet is dispatched.
- Keep Designer artifacts meso-scoped; do not ask Designers to pre-slice implementation passes.
- Every Creator-synced Checker pass mirrors its Creator pass exactly.
- Keep meso integration passes separate from Creator-synced Checker passes.
- When a structural cleanup pass exists, list each cleanup objective separately and record whether it is fully closed or intentionally deferred.

## Current Pass Slicing Decisions

- **`stage_d_scope_classification_20260730`**
  - **Kind**: Design-scope classification; not a Creator, Checker, Reviewer,
    or implementation pass.
  - **Parent**: `N/A` (global Stage-D decision across the accepted seven-Meso
    topology).
  - **Covered micro-features**: All 81 formal Micro IDs in
    `priors/MICRO_FEATURE_INVENTORY.md`.
  - **Decision**: `需要实现` contains all P0/P1 IDs plus `P2-01 xino` (56
    total). `暂不实现` contains `P2-02..P2-17` and `P3-01..P3-09` (25
    total).
  - **Rationale**: Stage D defines the basic implementation commitment as the
    complete P0/P1 foundation plus core xino identity projection. Every other
    P2/P3 item remains a deliberately mapped future insertion point, even when
    its dependency chain is high priority after the basic implementation.
  - **Explicit boundary**: This decision authorizes no Designer, Creator,
    Checker, Reviewer, build, test, or runtime work and creates no pass slices.
    Future pass slicing must be recorded only after an explicit Designer wave
    under the seven accepted Meso boundaries.

- **`topology_reset_20260730_reframed_architect`**
  - **Kind**: Architect design/reset wave; not an implementation, Creator,
    Checker, Reviewer, or pass-slicing pass.
  - **Parent**: `N/A` (fresh global decomposition).
  - **Covered micro-features**: All 81 formal Micro IDs in
    `priors/MICRO_FEATURE_INVENTORY.md`.
  - **Write-set**: Delete the superseded Architect/Meso/Designer dispatch
    artifacts; create one fresh Macro artifact and seven fresh Meso
    architecture maps under `components/architect-reframed-topology/`.
  - **Result**: Accepted after structural audit. Each formal Micro ID has one
    primary Meso owner; the new topology is `DIR -> CUL -> INODE -> WL ->
    UPPER` with out-of-band `IU` mount claims.
  - **Explicit boundary**: No Designer spec/validation, Creator/Checker/
    Reviewer packet, implementation pass, test, build, or runtime work is
    authorized by this decision. A future Designer wave must be separately
    dispatched against the seven new Meso boundaries.

- **`pass_00_old_ovfs_baseline_test`**
  - **Kind**: Pre-design legacy baseline Checker pass using the authorized overlay xfstests lane.
  - **Parent**: `legacy_overlayfs_baseline_validation` (temporary validation parent; not an accepted Architect meso-component).
  - **Covered micro-features**: `P0-01`, `P0-02`, `P0-03`, `P0-04`, `P0-05`, `P0-08`, `P0-09`, `P0-10`, `P0-11`, `P0-12`, `P0-14`, `P0-15`, `P0-18`, `P1-02`, `P1-03`, `P1-04`, `P1-06`, `P1-07`, `P1-08`, `P1-10`, `P1-12`, `P1-13`, `P1-16`, `P1-18`, `P1-21`, `P1-22`, `P1-23`, `P1-24`, `P1-25`, `P1-26`, `P1-27`, `P1-28`, `P1-29`, `P1-30`, `P1-31`, `P1-32`, `P1-34`, `P2-01`, `P2-02`, `P2-06`, `P2-07`, `P2-11`, `P2-12`, `P2-13`, `P2-14`, `P3-01`, `P3-02`, `P3-03`, `P3-04`, `P3-05`, `P3-08`, `P3-09`.
  - **Test scope**: every case listed by `test/initramfs/src/conformance/xfstests/overlay/run_list/full.list`; `overlay/100` and `overlay/101` remain case-matrix targets without a staged micro-feature mapping.
  - **Rationale**: establish a case-by-case legacy behavior matrix before the Architect converts the staged priors into the authoritative refactor topology. This pass has no Creator-synchronized companion and does not accept implementation work.
  - **Execution boundary**: one fresh QEMU and freshly recreated TEST/SCRATCH image pair per case; terminate at 300 seconds as `HANG/TIMEOUT`; classify exclusively from that case's `qemu.log` (`PASS`, `FAIL`, or explicit xfstests `[not run]` as `NOTRUN`); do not use `CORRUPT`; append the result before starting another case.
  - **Artifact boundary**: keep one reusable temporary single-case runlist only, keep one compact case-status table, and delete the temporary runlist after the baseline. Do not retain per-case logs or full per-case commands.

**Deferred / Exit Notes:**

- The baseline Checker report preserves execution evidence for all target
  cases. The original Architect wave was not dispatched automatically by that
  baseline task; it was later superseded by the explicitly dispatched
  `topology_reset_20260730_reframed_architect` wave above.
