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

- **`stage_d_scope_amendment_20260801_p2_11_uuid`**
  - **Kind**: Scope amendment to `stage_d_scope_classification_20260730`;
    not a Creator/Checker/Reviewer pass.
  - **Decision**: `P2-11 UUID modes` is promoted from `暂不实现` to `需要实现`
    in `mount_resource_policy` (user decision 2026-08-01). The overlay UUID and
    the upper/workdir claim token are unified as one 64-bit entity: when uuid
    policy is effective the same value is the persisted `overlay.uuid` and the
    claim token; otherwise only a fresh per-mount claim token exists.
  - **Resulting classification**: 57 `需要实现` / 24 `暂不实现`.
  - **Explicit boundary**: No pass slices are created by this amendment;
    pass slicing still awaits accepted Designer contracts.

- **`mount_validation_deferral_20260801`**
  - **Kind**: Scheduling decision; not a pass.
  - **Parent**: `mount_resource_policy` (Meso 01).
  - **Decision**: The Creator-synced RUNTIME Checker for meso 01 is deferred.
    Acceptance of the meso 01 Creator pass = structural acceptance + compile
    preflight only. Runtime xfstests validation of the mount group becomes a
    meso-integration obligation scheduled after `visibility_projection_identity`
    provides the root carrier and a minimal read path (mount -> root ->
    lookup/stat/readdir), because `FsType::create` cannot complete without the
    root-carrier seam (construction step 10) and most cases additionally need
    sibling-Meso lookup/readdir/IO behavior.
  - **Boundary**: No ktest/other lane is substituted; the validation contract's
    mapping table remains the target evidence contract; the failure-path
    subset may be reported at integration time only for cases that provably
    run without a successful baseline mount (Checker confirms from suite
    source).

- **`creator_pass_slicing_20260803`**
  - **Kind**: Pass-slicing decision (main-agent-owned) opening Phase 4 — the first
    Creator implementation wave; supersedes the "no pass slices yet" state of the
    Designer tenure.
  - **Workflow amendment (user-directed 2026-08-03)**: the test flow is
    xfstests-integration-only. **Creator-synced per-pass Checker passes are
    eliminated** for this wave; the Reviewer is the only quality gate before the
    code is complete (static review; PROTOCOL §1 rule 16 pre-checker structural
    audit, explicitly user-requested); the **meso-integration xfstests Checker**
    is the single runtime validation gate, scheduled after implementation +
    Reviewer stabilize. Creator passes are command-free and receive **no per-pass
    compile preflight**. This amends PROTOCOL §1 rule 5 for this wave by user
    direction; PROTOCOL.md is not edited unless the user confirms a permanent
    amendment.
  - **Slicing rule**: each Creator pass owns a disjoint write-set; shared-file
    edits (crate-root `overlayfs/mod.rs`, `mount/superblock.rs`,
    `mount/build.rs`, `projection/inode.rs`, `projection/entry.rs`,
    `projection/binding_cache.rs`, `projection/mod.rs`, `projection/identity.rs`)
    are serialized and their frozen cross-meso extensions/widenings consolidated
    into one seam-placement pass, so the four meso leaf passes are write-disjoint
    and run in parallel. All seams taken verbatim from the accepted meso-03/04/05/06
    consumption-seam records; no Designer contract is changed; the seams pass
    claims no micro-feature.
  - **Passes (waves 1-3 serial, wave 4 parallel, 57-micro union = Stage-D
    `需要实现` exactly):**
    - **Wave 1** `pass_01_mount_resource_policy` — parent
      `mount_resource_policy` (9/6): P0-01, P0-02, P0-03, P0-05, P0-18, P1-19,
      P1-20, P1-35, P2-11. Write-set: `overlayfs/mount/*` (new) + crate-root
      `overlayfs/mod.rs` (`mod mount;`). Risk High.
    - **Wave 2** `pass_02_visibility_projection_identity` — parent
      `visibility_projection_identity` (12/2): P0-04, P0-06, P0-07, P0-08,
      P0-09, P0-10, P0-11, P0-12, P0-16, P0-17, P1-07, P2-01. Write-set:
      `overlayfs/projection/*` (new) + `mount/superblock.rs` +
      `mount/build.rs` (meso-02 `OverlayFs` field additions + `OverlayFs::new`
      extension) + crate-root `overlayfs/mod.rs` (`mod projection;`). Risk High.
      Serial after Wave 1 (shared `mount/superblock.rs`/`build.rs`).
    - **Wave 3** `pass_03_shared_carrier_seams` — parent N/A (cross-meso shared
      carriers; foundation; **no feature claims**). Write-set: `mount/superblock.rs`
      (`workdir_temp_serial` meso-04, `xattr_policy` meso-05, `whiteout_cache`
      meso-06), `mount/build.rs` (their `OverlayFs::new` init), `projection/inode.rs`
      (`readdir_index` meso-03 + `copyup_transition` meso-04 fields + init;
      `facts_snapshot`/`dir` → `pub(super)`; `OverlayObjectFacts` content
      readable `pub(super)`), `projection/entry.rs` (`RealObject::new` +
      `is_opaque_directory` → `pub(super)`), `projection/binding_cache.rs`
      (`BindingKey::new` + positive/hidden construction), `projection/mod.rs`
      (`project_new_upper` seam + `record_copyup_transition` hook call at
      positive-binding assembly), crate-root `overlayfs/mod.rs` (declare
      `mod readdir_index; mod copyup; mod metadata_security; mod dir;` +
      shared `AccessType` enum). Risk Normal. Not a meso pass — recorded here so
      later main agents do not treat it as one.
    - **Wave 4 (PARALLEL — disjoint write-sets):**
      - `pass_04_merged_directory_index` — parent `merged_directory_index` (4/1):
        P0-13, P0-14, P0-15, P1-31. Write-set: `overlayfs/readdir_index.rs`
        (new; includes the meso-03-owned seams `invalidate_readdir_index`,
        `readdir_index_insert`/`readdir_index_remove`, `visible_child_count`).
        Risk High.
      - `pass_05_copyup_authority_file_views` — parent
        `copyup_authority_file_views` (17/6): P1-01, P1-02, P1-03, P1-04, P1-05,
        P1-06, P1-08, P1-09, P1-10, P1-11, P1-12, P1-13, P1-14, P1-15, P1-32,
        P1-34, P1-37. Write-set: `overlayfs/copyup/{mod.rs,coordination.rs,
        trigger.rs,promote.rs,workdir.rs}` (new). Risk High.
      - `pass_06_metadata_security_xattr_policy` — parent
        `metadata_security_xattr_policy` (4/4): P1-16, P1-17, P1-18, P1-33.
        Write-set: `overlayfs/metadata_security/{mod.rs,permission.rs,
        metadata.rs,xattr.rs}` (new). Risk High.
      - `pass_07_namespace_mutation_whiteout` — parent
        `namespace_mutation_whiteout` (11/1): P1-21, P1-22, P1-23, P1-24,
        P1-25, P1-26, P1-27, P1-28, P1-29, P1-30, P1-36. Write-set:
        `overlayfs/dir/{mod.rs,create.rs,remove.rs,link.rs,rename.rs,
        whiteout.rs}` (new). Risk High.
  - **Post-implementation gates (workflow amendment):** per-meso Reviewer wave
    (static only; the sole pre-code-completion gate), then a meso-integration
    Checker wave (compile + full kernel build + overlay xfstests suite per the
    six Designer validation contracts; the single runtime gate). Deferred
    validation rows from the meso-01..06 validation contracts resolve there.
  - **Legacy-reference boundary (user-directed 2026-08-03):** the old
    single-file implementation was renamed `fs.rs` → `legacy_fs.rs` (git rename;
    content unchanged except a LEGACY/FROZEN header banner; `mod.rs` updated to
    `mod legacy_fs; use legacy_fs::OverlayFsType;`). The legacy `OverlayFsType`
    registration in `overlayfs::init()` remains the ACTIVE registered overlay
    filesystem until an explicit takeover decision after the integration gate.
    Every Creator/Reviewer packet MUST state that the only permitted reference
    to `legacy_fs.rs` is the registration wiring (`OverlayFsType` `FsType` impl
    + `register()` shape); all other legacy content is forbidden as a reference
    — implementation sources are the Designer specs, `designdoc/`, and
    `priors/` only.
  - **Explicit boundary:** `暂不实现` IDs (24) remain untouched; no ktest surface
    is created or modified; no `#[ktest]`/`#[cfg(ktest)]` surface in
    `legacy_fs.rs` is touched by the rename.
  - **Subagent orchestration (user-confirmed 2026-08-03; full text in the live
    handoff §2A):** one Creator per `.rs` file per Wave; Wave-end review via
    aster-code-review in **`diff` mode** (base = previous accepted Wave commit;
    3-persona fan-out as direct subagents, no `codex exec`); long-lived repair
    Reviewer+Creator loop with `--amend` per round; per-Wave commits amended
    during repair and stabilized at acceptance; review packets/outputs under the
    gitignored `subagent-tasks/` / `components/` trees; durable `.agents/*.md`
    records stay uncommitted during the Wave cycle so review diffs are `.rs`-only.
