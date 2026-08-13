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

- **`pre_wave5_c2_required_20260804`**
  - **Kind**: Scheduling and design-revision decision; executed and accepted
    through the bounded closure passes below.
  - **Parent**: `copyup_authority_file_views` owns the stable workdir-temp
    request/retry interface (`P1-34`); the namespace-mutation consumers are
    explicitly listed below and do not change Meso ownership.
  - **Decision**: Complete bounded `EEXIST` retry for workdir temps (C2) is a
    **required pre-wave5 correctness repair**, superseding the 2026-08-04
    deferral recorded in the prior handoff state. Wave5 is blocked until the
    required design and implementation receipts are accepted.
  - **Required design before dispatch**: a bounded Designer addendum must
    freeze one typed request/result interface in `copyup/workdir.rs`, one
    shared retry bound, fresh-name-per-attempt behavior, retry-on-`EEXIST`
    only, error propagation for every other failure, and the exact use of the
    successful `(name, inode)` pair by every caller. It must not use an opaque
    closure-based helper.
  - **Required implementation scope**: `copyup/promote.rs`,
    `copyup/workdir.rs`, `dir/remove.rs`, `dir/create.rs`, `dir/link.rs`, and
    `dir/whiteout.rs`. The packet must enumerate every workdir-temp creation,
    link, and mknod call site in those files. A two-site or random-suffix-only
    repair is rejected.
  - **Validation and boundary**: no ktest surface; no pre-wave5 build or
    runtime command. Later meso-integration xfstests remains the sole runtime
    lane. This decision does not reopen deferred P1 overlay-parent identity or
    P2 executable creator credentials.
  - **Designer acceptance (2026-08-04)**: the bounded addendum under
    `components/pre_wave5_closure/` is structurally accepted. It freezes the
    v3 origin-wire/snapshot contract, the complete six-file C2 retry contract,
    and six mechanical dispositions.

- **`pre_wave5_closure_creator_slicing_20260804`**
  - **Kind**: Main-agent Creator slicing for the accepted bounded closure
    addendum. The three phases are serial because their write-sets and the
    final `mount/build.rs` cleanup overlap.
  - **User-directed execution rule**: one long-lived Creator agent executes
    all phases in order. After each phase, the main agent reviews its exact
    production diff; any finding returns to that same Creator for an in-place
    repair. The main agent amends only the accepted phase's Rust write-set.
    This is not a new Reviewer wave and does not reintroduce the prior
    Reviewer loop.
  - **Phase A — `pass_08_workdir_temp_retry`**
    - **Parent**: `copyup_authority_file_views`.
    - **Covered Micro-Features**: `P1-34`.
    - **Write-set**: `copyup/{workdir.rs,promote.rs}` and
      `dir/{remove.rs,create.rs,link.rs,whiteout.rs}`.
    - **Scope**: complete typed `WorkdirTempRequest`/`WorkdirTemp` retry
      interface, all six call sites, fresh-name-per-attempt, the shared bound,
      and `EEXIST`-only retry. Risk High.
  - **Phase B — `pass_09_origin_triplet`**
    - **Parent**: `visibility_projection_identity`.
    - **Covered Micro-Features**: `P1-07`.
    - **Write-set**: `mount/build.rs`, `projection/identity.rs`,
      `projection/lower_id.rs`, and the inseparable record-consumer propagation
      in `readdir_index.rs`.
    - **Scope**: native v3 `(container_dev_id, lower_layer_root_ino,
      real_ino)` wire, immutable lower snapshot, unique-fsid resolution, and
      conservative fallback. This phase also closes the inseparable sixth
      mechanical item by removing the never-read `IdentityPolicy::layer_devs`.
      Risk High.
  - **Phase C — `pass_10_closure_mechanical`**
    - **Parent**: N/A — user-directed bounded cross-meso cleanup; `P0-15` is
      documentation-only and the remaining repairs claim no Micro-feature.
    - **Covered Micro-Features**: `P0-15` documentation-only; no new feature
      claim for the five mechanical objectives.
    - **Write-set**: `readdir_index.rs`, `dir/remove.rs`,
      `metadata_security/xattr.rs`, and `mount/build.rs`.
    - **Scope**: C1 weaker index/facts documentation, five-arm
      `parent_fallback` documentation, clear-empty xattr wording, associated
      xattr helpers, and explicit `if let` in `mount/build.rs`. The
      `IdentityPolicy::layer_devs` objective is already closed in Phase B and
      is not re-edited. Risk Normal.
  - **Shared boundary**: Creator phases are command-free, no ktest surface is
    permitted, and no P1/P2/generation/fingerprint/VFS or lock-topology work
    may enter a phase. After Phase C is accepted and amended, the main agent
    records closure and opens the Wave5 Checker-owned compile/lint lane.
  - **Execution / acceptance (2026-08-04)**: one long-lived Creator completed
    all three phases; the main agent reviewed each exact Rust diff and routed
    two P3 documentation/formatting corrections back to that same Creator.
    Phase A accepted the six-file typed retry; Phase B accepted the v3 triplet,
    lower snapshot, unique pair resolver, `readdir_index.rs` consumer update,
    and `IdentityPolicy::layer_devs` removal; Phase C accepted all five
    remaining mechanical repairs. Each accepted Rust write-set amended the
    same commit, now `7aabd029c` (`Add pre-wave5 bounded overlayfs revisions`),
    whose title contains no `WIP`. No Reviewer was introduced.

- **`wave5_compile_lint_20260804`**
  - **Kind**: Checker-owned static meso-integration entry lane.
  - **Parent**: `overlayfs_refactor_static_integration` (temporary validation
    parent; not an accepted Architect meso-component).
  - **Covered Micro-Features**: all 57 Stage-D `需要实现` IDs, including the
    closure-revised `P1-07`, `P1-34`, and documentation-only `P0-15`; the
    remaining mechanical repairs claim no new Micro-feature.
  - **Dispatch**: `task_checker_wave5_compile_lint_20260804` through
    `subagent-tasks/wave_05_compile_lint/pass_11_wave5_compile_lint_checker_dispatch.md`.
  - **Scope**: serialized container-only `cargo check` → `make kernel` →
    `make check`. A failure produces a preserved-evidence actionable repair
    batch; the Checker changes no production source. This is not the later
    xfstests integration Checker and introduces no Reviewer or ktest surface.
  - **Initial Checker result (2026-08-04)**: the target-specific `cargo check`
    failed with 42 errors (exit 101), so `make kernel` and `make check` were
    not run. Evidence and the repair batch are in
    `components/wave_05_compile_lint/pass_11_wave5_compile_lint_checker.md`.
    Before any bounded overlayfs repair, the VFS owner must decide the narrow
    `FsCreationCtx` task-context accessor and the in-use claim-slot facility;
    the Checker prohibits faking either facility inside overlayfs.
  - **Takeover continuation (2026-08-04, user-directed):** before the next
    Checker run, `overlayfs::init()` was switched to register
    `mount::OverlayFsType`, the deferred `dead_code` expectation was removed,
    and `mod legacy_fs;` wiring was removed while retaining `legacy_fs.rs` as
    an unlinked archive. These two Rust files amended the same commit, now
    `73aec1ae3`. Continuation packet
    `pass_11_wave5_compile_lint_checker_continuation_01_dispatch.md` authorizes
    only the exact protocol `cargo check` command. Its failure triage permits
    main-agent repair only for mechanical spelling/import/visibility/interface
    propagation; any ownership, VFS contract, lifecycle, locking, semantic, or
    non-obvious borrow repair waits for user direction.
  - **Initial static stop record (2026-08-04): superseded by the accepted
    user decisions and implementation below.** Three
    continuations ran only the same prescribed target-specific `cargo check`.
    The main agent amended every Checker-classified mechanical repair, reducing
    the result from 42 errors / 16 warnings to 15 errors / 1 warning at
    `7aabd029c`. No mechanical candidate remains. The unresolved categories
    are the root-inode one-shot carrier, `OverlayInode` trait-implementation
    ownership, `FsCreationCtx` task-context access, in-use claim-slot
    lifecycle, and the source-permission `AccessType`. At this point the
    static-entry cargo stage stopped; `make kernel`, `make check`, runtime,
    and xfstests remained unscheduled pending those decisions.

- **`wave5_static_owner_reconciliation_design_20260804`**
  - **Kind**: Bounded Designer code-form revision; not a Creator, Checker,
    Reviewer, or implementation pass.
  - **Parent**: `N/A` — preserves the existing Meso owners while reconciling
    the Wave5 static-entry blockers.
  - **Covered Micro-Features**: `P0-01`, `P0-05`, `P1-16`, `P1-19`, and
    `P1-35`, plus the already-landed trait-required surfaces; no new
    Micro-feature claim.
  - **Dispatch / acceptance**: `task_designer_wave5_static_owner_reconciliation_20260804`,
    accepted 2026-08-04. Its packet and exactly two Designer artifacts live
    under `subagent-tasks/wave_05_static_repair_design/` and
    `components/wave_05_static_repair_design/`.
  - **Decision**: Freeze the five user-adjudicated forms: a private
    `Mutex<Option<Arc<dyn Inode>>>` root publication slot; sole `Inode` and
    `FileOps` implementations in `projection/inode.rs` forwarding to
    existing-Meso `*_impl` helpers; `FsCreationCtx::task_ctx()`; a dedicated
    `Extension` group whose `overlay_inuse_slot()` accessor lazily initializes
    one token-only atomic slot; and the `ReadOnly` source link admission.
  - **VFS boundary**: exact expected write-set is
    `kernel/core/src/fs/vfs/fs_apis/{registry.rs,inode.rs,inode_ext.rs}`. It adds no
    VFS module, global map, other extension-group use, context carrier, or
    lock domain. The slot's `Acquire`/`Release`/`Relaxed` protocol protects
    only token ownership; existing `InodeClaimGuard` keeps the inode pin and
    releases only its own token.
  - **Explicit boundary**: this acceptance authorizes neither a Creator
    packet nor any command. The next implementation decision must separately
    choose and packet the disjoint VFS seam and Overlayfs forwarding/root/
    permission write-sets; P1/P2 deferrals remain untouched.

- **`wave5_compile_lint_continuation_04_20260804`**
  - **Kind**: One-run Checker continuation under
    `wave5_compile_lint_20260804`; not a new pass or implementation repair.
  - **Parent / covered scope**: unchanged
    `overlayfs_refactor_static_integration` / all 57 Stage-D `需要实现` IDs.
  - **Dispatch**: exactly one existing target-specific container `cargo check`
    after amend `783c81041`; packet
    `pass_11_wave5_compile_lint_checker_continuation_04_designer_reconciliation_dispatch.md`.
  - **Boundary**: no production code is authorized or implied by the accepted
    Designer documents. The Checker preserves a new raw receipt and reports
    the original diagnostics; `make`, Clippy, runtime, xfstests, and a
    self-directed repair or rerun remain out of scope.
  - **Result**: the one run at `783c81041` exited `101` with 15 errors and one
    warning, unchanged from continuation 03. The five user-decided forms are
    still unimplemented, so no diagnostic is a newly authorized mechanical
    repair and this continuation stops.

- **`wave5_static_owner_creator_slicing_20260804`**
  - **Kind**: Main-agent Creator slicing of the accepted five-item static
    owner reconciliation.
  - **Shared boundary**: each Creator is command-free and writes only its
    exact production set plus one ignored receipt. The main agent reviews each
    completed exact diff; no Checker, `make`, Clippy, runtime, or xfstests run
    occurs until the five Creator surfaces are accepted together.
  - **Pass 12 — `root_publication`**: parent `mount_resource_policy`, `P0-05`;
    `mount/{superblock.rs,build.rs}`. Replaces only the root `OnceLock` with
    the accepted mutex-option publication slot. **Accepted by main-agent exact
    diff review:** no new entity; root materializes before the short slot lock,
    publication is `Some(root)` before return, and the getter only clones.
  - **Pass 13 — `trait_owner`**: parent `N/A` (user-directed bounded
    cross-Meso reconciliation; no new Micro claim);
    `projection/inode.rs`, `copyup/mod.rs`, `readdir_index.rs`,
    `metadata_security/{metadata.rs,permission.rs,xattr.rs}`, and `dir/mod.rs`.
    Centralizes the sole trait implementations and leaves body helpers in
    their existing Meso files. **Accepted by main-agent exact diff review:**
    `projection/inode.rs` is the sole `Inode`/`FileOps` carrier; all sibling
    bodies are direct `*_impl` helpers, and corrected ownership documentation
    matches that split.
  - **Pass 14 — `task_context`**: parent `mount_resource_policy`, `P0-01` and
    `P1-19`; VFS `registry.rs` plus `mount/{build.rs,layers.rs}`. It begins
    only after pass 12 acceptance because both touch `build.rs`. **Accepted by
    main-agent exact diff review:** only `registry.rs` changes, adding the
    narrow immutable field borrow; existing mount consumers do not retain a
    `Context` or alter construction sequencing.
  - **Pass 15 — `inuse_slot`**: parent `mount_resource_policy`, `P1-35`; VFS
    `{inode.rs,inode_ext.rs}` plus `mount/claims.rs`. It is write-disjoint from
    passes 12/13 and may begin with them. **Accepted by main-agent exact diff
    review:** group3 is dedicated, the accessor is the only lazy initializer,
    group1/group2 and lock topology are unchanged, and the pinned guard
    releases only its own token with the frozen atomic ordering.
  - **Pass 16 — `link_source_permission`**: parent
    `namespace_mutation_whiteout`, `P1-28`; `dir/mod.rs` only. It begins only
    after pass 13 acceptance because the canonical `link_impl` helper lands
    there. **Accepted by main-agent exact diff review:** before `link_source`
    can promote, the unchanged owner/DAC predicate calls the inherent
    `check_permission(AccessType::ReadOnly, Permission::MAY_WRITE)` admission;
    the target parent remains `Mutating`.
  - **Commit boundary**: after all five independent review acceptances, the
    main agent amends only their accepted Rust write-sets into the current
    Wave5 commit; durable scheduler records stay uncommitted until a later
    explicit integration/acceptance commit.
  - **Implementation / continuation 05 (2026-08-04):** all five passes were
    accepted and the 13 Rust-file implementation amended at `10cf627e2`.
    The one authorized continuation-05 cargo check failed with five errors
    and one warning; its raw evidence is in
    `components/wave_05_compile_lint/run_evidence/20260804T_continuation_05_static_owner_implementation_dispatch/`.
    The main agent amended the three checker-classified mechanical candidates
    (two scope imports and the underscore unused-local rename) to
    `1378d502a`, without a second command.
  - **Pass 17 / continuations 06-07 (2026-08-04):** after user direction, the
    command-free Creator made exactly three no-clone move-order repairs in
    `projection/entry.rs` and `readdir_index.rs`, preserving the upper opaque
    short-circuit, lower merged-layer insertion, and captured readdir type.
    Main-agent exact-diff review accepted the repair and amended `90a5facf7`.
    The single continuation-06 cargo check then exited `101` with those
    `E0382`s absent and only two claim-type visibility errors plus 17 warnings.
    The main agent applied the direct existing-ceiling visibility propagation
    for `UpperWorkdirClaim` only and amended `36c30ac33`. The single
    continuation-07 cargo check exited `0` in 8.54 seconds; evidence is in
    `components/wave_05_compile_lint/pass_11_wave5_compile_lint_checker.md`
    §12. The 17 warnings remain recorded warning debt. This proves only the
    prescribed cargo smoke: `make kernel`, `make check`, runtime, and xfstests
    remain unrun and require separately packeted Checker work.

- **`wave5_policy_binding_lint_cleanup_20260804`**
  - **Kind**: User-approved bounded cross-Meso lint-cleanup continuation.
  - **Parent / covered scope**: `N/A`; no new Micro-feature claim. It is the
    representation-only disposition of the two remaining non-documentation
    Clippy diagnostics after continuation 10.
  - **Designer acceptance**: `task_designer_wave5_policy_binding_lint_20260804`
    freezes one non-escaping `&OverlayMountOptions` input to
    `MountPolicy::assemble` and two private aliases naming the existing
    `parent -> name -> binding` maps. No owner, lock, allocation, cache key,
    policy field, or lifecycle change is permitted.
  - **Creator Pass 21 — `policy_binding_lint_cleanup`**: exact Rust write-set
    `mount/{policy.rs,build.rs}` and `projection/binding_cache.rs`; no test,
    VFS, configuration, `legacy_fs.rs`, or documentation-lint path. The
    policy constructor copies only `uuid_mode`, `is_default_permissions`, and
    `xino_mode` from the construction-local options borrow; its other five
    existing inputs remain individual. `BindingsByName` and
    `BindingsByParent` are private aliases only.
  - **Validation boundary**: Creator is command-free. The existing Wave5
    static Checker task may continue after the exact three-file pass with the
    prescribed cargo check -> make kernel -> workspace Clippy sequence. rustfmt
    and `make check` remain deferred until the user-directed Wave6 document
    cleanup is complete.

- **`wave6_documentation_review_20260805`**
  - **Kind**: Main-agent pass-slicing decision (user-directed, 2026-08-05)
    opening Wave6 as a **comprehensive comment-documentation review**, not
    just the nine Clippy diagnostics. Supersedes the lint-only framing of the
    prior handoff status; the nine diagnostics and the two user-required TODO
    annotations are mandatory items inside two of the six passes below, not
    separate passes.
  - **Scope**: every active overlayfs source file (all 31 non-`legacy_fs.rs`
    `.rs` files) is comment-rewritten so a developer reading only
    `kernel/core/src/fs/fs_impls/overlayfs/` understands the code without the
    `.agents/` workspace. All micro-feature IDs (`P0-xx`/`P1-xx`/`P2-xx`/
    `P3-xx`) and internal workspace vocabulary (Meso/Wave/pass IDs/spec §/
    review-repair-round history/RECONCILIATION/frozen etc.) are removed from
    comments; stale and redundant prose is rewritten against current code;
    the nine rustdoc diagnostics in `mount/build.rs:44-50` and
    `dir/remove.rs:76-77` are fixed; exactly two scoped comment-only TODO
    annotations are added (one in `mount/build.rs`, one in `dir/remove.rs`).
    Normative rules: `subagent-tasks/wave_06_documentation_review/
    WAVE6_DOC_STANDARD.md`.
  - **Passes (six, one per implemented meso, disjoint write-sets; batch A =
    passes 22-24, batch B = passes 25-27; all command-free, risk Low):**
    - **Pass 22** `mount_resource_policy` — write-set `mount/*` +
      crate-root `mod.rs` annex (parent N/A for the annex; `AccessType` docs
      only). Includes the 7 `build.rs` diagnostics + TODO 1. Covered
      micro-features: P0-01, P0-02, P0-03, P0-05, P0-18, P1-19, P1-20,
      P1-35, P2-11.
    - **Pass 23** `visibility_projection_identity` — write-set `projection/*`.
      Covered: P0-04, P0-06, P0-07, P0-08, P0-09, P0-10, P0-11, P0-12,
      P0-16, P0-17, P1-07, P2-01.
    - **Pass 24** `namespace_mutation_whiteout` — write-set `dir/*`. Includes
      the 2 `remove.rs` diagnostics + TODO 2. Covered: P1-21..P1-30, P1-36.
    - **Pass 25** `copyup_authority_file_views` — write-set `copyup/*`.
      Covered: P1-01..P1-06, P1-08..P1-15, P1-32, P1-34, P1-37.
    - **Pass 26** `metadata_security_xattr_policy` — write-set
      `metadata_security/*`. Covered: P1-16, P1-17, P1-18, P1-33.
    - **Pass 27** `merged_directory_index` — write-set `readdir_index.rs`.
      Covered: P0-13, P0-14, P0-15, P1-31.
  - **Acceptance flow**: main-agent exact-diff review per lane (comment-only;
    reject any behavioral, signature, formatting-outside-comments, `#[allow]`/
    `#[expect]`, `legacy_fs.rs`, VFS, or test change). After all six lanes
    are accepted, the Checker runs workspace Clippy; only on clean Clippy it
    runs rustfmt, then `make check`, preserving evidence per run.
  - **Static closure (2026-08-05, ACCEPTED)**: all six Creator lanes were
    executed in-thread (subagent task delivery failed in the session) and
    accepted; workspace Clippy, `cargo fmt --check`, and `make check` all
    exit 0. The pre-existing workspace format drift (incl. VFS
    `inode_ext.rs`) and trailing whitespace in 11 pre-existing `.agents/`
    markdown files were cleared by the authorized mechanical `cargo fmt` and
    whitespace stripping; receipts and run evidence are under
    `components/wave_06_documentation_review/`.
  - **Second review pass (2026-08-05)**: user-directed cleanup of doc comments
    that focused on mechanical details (`pub`/visibility, `#[derive(Debug)]`,
    seam/ceiling/accessor phrasing) instead of meaning and design intent.
    Comment-only across all six lanes; revalidated by workspace Clippy,
    `cargo fmt --check`, and `make check` (runs 06-08, all exit 0).
  - **Explicit boundary**: no production behavior, owner, lock, cache, VFS,
    test, harness, xfstests, or `legacy_fs.rs` edit. P1/P2 deferrals, origin
    UUID/export-FH parity, P2-07/P3-01, runtime xfstests, final Reviewer
    acceptance, and Wave7 `legacy_fs.rs` deletion remain outside Wave6.

- **`wave7_logic_bug_repair_20260807`**
  - **Kind**: User-directed bounded repair wave (Designer addendum + Creator
    implementation) fixing the five Wave7 xfstests logic-bug groups. Parent
    meso owners are preserved; no Architect repair is required (no static
    owner or lock-topology defect was found). No ktest surface; no xfstests
    run is scheduled by this decision — the user directed the main agent to
    verify compilation only after the Creator lands.
  - **Designer decision**: `task_designer_wave7_logic_bug_repair_20260807` —
    one bounded cross-meso repair addendum under
    `components/wave7_logic_bug_repair/` (one spec + one validation file),
    preserving every existing Meso owner and the accepted static topology.
  - **Creator slicing**: one Creator pass
    `pass_28_wave7_logic_bug_repair` (Risk High) with an enumerated
    write-set. Passes are executed after the Designer addendum is accepted.
  - **Repair objectives (each parent Meso + covered Micro, from the Wave7
    batch summary and upstream suite sources):**
    1. **Workdir residue cleanup (`overlay/024`, `overlay/010`)** — parent
       `mount_resource_policy` (Meso 01), covered `P0-03` (workdir setup).
       `prepare_workdir` (`mount/claims.rs`) must accept a non-empty workdir
       root and clean the `<workdir>/work` residue at mount, instead of
       failing `ENOTEMPTY` (Linux `ovl_get_workdir`/`ovl_make_workdir`/
       `ovl_workdir_cleanup` semantics).
    2. **Readdir lower opaque layer barrier (`overlay/014`)** — parent
       `merged_directory_index` (Meso 03), covered `P0-14` (merged readdir).
       `readdir_sequence` (`readdir_index.rs`) currently checks the opaque
       barrier on the upper layer only; it must stop the downward merge at
       the first opaque layer (Linux `ovl_iterate` layer-stack stop).
    3. **`ETXTBSY` on truncate of an executing overlay file
       (`overlay/013`)** — parent `copyup_authority_file_views` (Meso 04),
       covered `P1-08` (file open / truncate real-file authority). The
       truncate path (`resize_impl`) must fail `ETXTBSY` when the target
       overlay file is currently executing. The priors document the lower
       `ETXTBSY` divergence (§19c) as preserved; the Designer decides the
       narrow VFS/process seam (exec mark + truncate check) with an explicit
       lifecycle (mark on successful exec load, release at process teardown).
    4. **`trusted.overlay.*` getxattr errno (`overlay/026`)** — parent
       `metadata_security_xattr_policy` (Meso 05), covered `P1-33` (xattr
       get/set/list delegation). The generic `get_xattr` path must return
       `EOPNOTSUPP` (Linux v4.10+ semantics) instead of `ENODATA` for
       non-`Public` overlay-private names.
    5. **Stale upper dentry removal `ESTALE` (`overlay/012`)** — parent
       `namespace_mutation_whiteout` (Meso 06), covered `P1-26` (unlink).
       When a positive projection asserted an upper object at `name` and the
       physical upper operation reports the object gone (`ENOENT`), the
       remove recipe must return `ESTALE` (Linux
       `ovl_remove_and_whiteout`/`ovl_verify_upper` semantics) instead of
       propagating `ENOENT`.
  - **Explicit boundary**: the pass touches production code only; no
    `legacy_fs.rs`, no test/harness files, no `SYSTEM_BLUEPRINT.md` state
    change beyond the repair record, and no xfstests/QEMU run. The main
    agent personally verifies the target-specific `cargo check` after the
    Creator reports (user-directed; no experiment restart).

- **`wave7_workdir_workspace_revision_20260807`**
  - **Kind**: User-directed bounded revision of the wave7 repair O1
    objective; Designer audit + Creator implementation. Supersedes the
    "staging at the workdir root" disposition of
    `wave7_logic_bug_repair_20260807` O1.
  - **Decision**: every workdir usage site must resolve to
    `<workdir>/work` (Linux `OVL_WORKDIR_NAME`, `ofs->workdir`) as the actual
    staging workspace, never the claimed workdir root. The claim protocol
    still claims the workdir ROOT inode; the staging workspace is pinned as a
    plain `Arc<dyn Inode>` on `UpperWorkdirClaim` after `prepare_workdir`
    (create when absent, residue-clean + recreate when present; never
    `ENOTEMPTY` for root residue). Mount order becomes prepare (step 7) →
    capability probes against the workspace (step 8) → UUID persist (step 9).
    `OverlayFs::workdir_root` (and the `OverlayInode` convenience) resolve
    the pinned workspace; all staging consumers (workdir temps, whiteout
    cache, copy-up/promote renames, dir create/link/remove recipes) follow
    without per-consumer edits.
  - **Designer acceptance**: `task_designer_wave7_workdir_workspace_audit_20260807`
    accepted 2026-08-07 (audit table + frozen Rust surface under
    `components/wave7_logic_bug_repair/wave7_workdir_workspace_designer_spec.md`
    + `_designer_validation.md`).
  - **Creator pass**: `pass_29_wave7_workdir_workspace` (Risk High,
    command-free), write-set
    `mount/{claims.rs,build.rs,policy.rs,superblock.rs}`,
    `copyup/{workdir.rs,promote.rs}`, `dir/{link.rs,remove.rs,whiteout.rs}`
    (doc-only for promote/link/remove/whiteout/superblock),
    `dir/create.rs` dispositioned no-edit. Accepted after main-agent diff
    review; compile verified by the main agent (user-directed).
  - **VFS revert (user-directed, 2026-08-07)**: the wave7 O3 `ETXTBSY`
    mechanism (VFS `Inode` trait `deny_write_access`/`allow_write_access`
    defaults + overlay count + process exec/fork/drop lifecycle) is
    **fully reverted** — interface-breaking VFS modifications are refused for
    this wave. `overlay/013`'s `ETXTBSY` behavior therefore remains a
    documented divergence (priors §19c) pending a redesign that does not
    modify VFS interfaces. All seven touched files
    (`fs/vfs/fs_apis/inode.rs`, `projection/{inode.rs,mod.rs}`,
    `copyup/mod.rs`, `process/{execve.rs,process_vm/mod.rs}`,
    `process/process/init_proc.rs`) are byte-identical to HEAD.
  - **Explicit boundary**: no VFS/process write-set for the remainder of this
    wave; no ktest surface; no xfstests/QEMU run scheduled by this decision;
    the main agent verified the target-specific `cargo check` (exit 0,
    pre-existing `MountPolicy::uuid_mode` warning only).

- **`pass_37_wave7_fixed_case_rerun`**
  - **Kind**: Main-agent Creator slicing for the accepted fixed-case-rerun
    Designer addendum (`task_designer_wave7_fixed_case_rerun_20260807`).
    Continuation of `wave7_logic_bug_repair_20260807`; no Architect repair
    (no static owner or lock-topology defect).
  - **Parent**: `mount_resource_policy` (Meso 01).
  - **Covered Micro-Features**: `P0-02` (layer stack assembly; lowerdir
    ordering parity), `P0-03` (workdir setup; workspace mode + mount-time
    cleanup coherence). The Meso-03 readdir opaque barrier (`P0-14`) is NOT
    re-sliced: the re-run proved it correct; `overlay/014` serves as external
    evidence for the ordering fix.
  - **Write-set (exact, per the accepted spec)**: `mount/options.rs`
    (doc corrections only), `mount/layers.rs` (delete
    `normalize_lower_ordering`; consume parsed `lower_dirs` directly; doc
    corrections), `mount/claims.rs` (new `WORKDIR_MODE` const = 0o700 +
    divergence doc; `prepare_workdir(&mut self, workdir_path: &Path)` with
    `Path::rmdir`/`Path::unlink`/`Path::new_fs_child` for the visible `work`
    name + `TODO(workdir-cleanup-vfs-parity)` / `TODO(workdir-mode)`),
    `mount/build.rs` (step 7 passes `&workdir_path`; doc note). Zero VFS
    interface change; no ktest surface; `remove_work_entries` raw recursion
    retained (residue dentry discarded wholesale).
  - **Execution / acceptance (2026-08-07)**: Creator executed in-thread (no
    subagent per user direction; the accepted spec is the packet). Accepted
    after main-agent exact-diff review; no deviations. User-directed
    compile verification (main-agent run): `cargo check -p aster-kernel
    --target x86_64-unknown-none` exits 0 with only the pre-existing
    `MountPolicy::uuid_mode` warning; `cargo fmt --check` on the four changed
    files exits 0. No QEMU/xfstests run (user owns the re-run decision).
  - **Boundary**: 014 opaque-marker/clear-empty triage and `overlay/013`
    `ETXTBSY` remain out of scope per the handoff.

- **`pass_39_wave7_comprehensive_cases_20260808`**
  - **Kind**: Meso-integration external-validation batch (Checker pass,
    main-agent executed in-thread per user direction; no subagents).
  - **Parent**: `overlayfs_refactor_meso_integration` (temporary integration
    parent).
  - **Covered Micro-Features (many-to-many)**: `P0-14`/whiteout visibility
    and `P1-2x` remove semantics (`overlay/031`), copy-up/impure dir
    (`overlay/038`), `P2-01` xino (`038` observed / `041` option gate),
    `d_real` stacked resolution (`029`), readdir cache invalidation (`077`),
    credential/namespace behavior (`020`, NOTRUN at userns gate).
  - **Execution shape**: one case per run via temporary
    `wave7-single.list`; images rebuilt fresh before each case; per-case
    `qemu.log`/`qemu-serial.log` archived under
    `components/wave7-xfstests-sequencing/run_evidence/<case>/comprehensive_20260808/`;
    upstream sources `020/031/038/041` archived under
    `run_evidence/upstream_sources/20260808/`; receipt
    `components/wave7-xfstests-sequencing/pass_39_wave7_comprehensive_cases_checker.md`.
  - **Result**: `029`/`077` PASS; `031` FAIL (invalid whiteout exposure after
    lowerdir change + merged-dir remove `ENOTEMPTY`); `038` FAIL (missing
    `trusted.overlay.impure` marker); `020` NOTRUN (unshare -m -p -U
    unsupported); `041` NOTRUN (`xino=on` mount option unsupported).
    Cross-run finding: 031 residue leaves a stale unmerged dirent record on
    base ext2 (whiteout char-device unlink path), breaking the next case's
    `_scratch_mkfs` cleanup with `rmdir ENOTEMPTY` on an on-disk-empty dir;
    every later case therefore ran on rebuilt images.
  - **Boundary**: `078` and stress cases `001`/`021`/`019` explicitly
    excluded by the user; `013` ETXTBSY still unscheduled; no ktest surface;
    no production code changed; temp runlist and generated images deleted
    after evidence capture.

- **`wave7_impure_cleanup_design_20260808`**
  - **Kind**: Main-agent design-research dispatch decision (Designer task,
    user-directed; dispatched via the WSL codex CLI launcher
    `aster-code-review/scripts/run_agent.sh` with the `codex` profile —
    private `CODEX_HOME` + inherited auth + `codex exec`, NOT the desktop
    subagent mechanism per user direction).
  - **Parent**: `N/A` — bounded cross-meso design research preserving every
    existing Meso owner; no Architect repair expected.
  - **Objectives**:
    1. **Impure marker persistence** (`overlay/038` FAIL) — enumerate every
       modification point to write/clear `trusted.overlay.impure` on upper
       dirs (copy-up-into-dir, whiteout publish, clear-empty, purity
       restoration), assign the marker owner, freeze constants/helpers/
       signatures, and record the private-filter interaction.
    2. **Whiteout cleanup before rmdir** (`overlay/031` FAIL) — branch census
       of every "visible-empty but physical-upper-non-empty" site
       (pure-upper rmdir arm confirmed by probe; lower-backed clear-empty;
       rename/whiteout adjacencies), freeze the repair shape (pre-rmdir
       whiteout sweep vs clear-empty routing), and explicitly disposition
       the never-exercised existing clear-empty implementation.
  - **Bug B boundary**: the base-fs↔overlayfs view-coherence fix is
    explicitly OUT OF SCOPE; the packet requires only dependency-edge notes.
  - **Packet**: `subagent-tasks/wave7_impure_cleanup_design/task_designer_wave7_impure_cleanup_20260808_dispatch.md`.
  - **Expected artifacts** (write-set, exactly two):
    `components/wave7_impure_cleanup_design/wave7_impure_cleanup_designer_spec.md`
    and `_designer_validation.md`.
  - **Boundary**: no production `.rs` edits, no commands, no ktest surface,
    no pass slicing, no VFS/base-view design, no task-board edits by the
    Designer.
  - **Acceptance (2026-08-08): ACCEPTED structurally.** Designer (via WSL
    codex CLI, inherited `~/.codex`, deepseek-v4-flash/custom) completed the
    research; artifacts written after the platform exec outage cleared:
    `components/wave7_impure_cleanup_design/wave7_impure_cleanup_designer_spec.md`
    (718 lines) and `_designer_validation.md` (136 lines). Both carry the
    template sections per objective; micro IDs reconciled to the inventory
    (`P2-03`, `P1-33`, `P1-01..P1-07`, `P1-25..P1-28`, `P1-31`, `P1-36`);
    Bug B appears only as out-of-scope dependency notes. Frozen surface:
    impure set triggers T1-T4 / clear C1-C2 +
    `OverlayXattrPolicy::{has,set,clear}_impure_marker` +
    `OverlayInode::refresh_impure_marker` + `IMPURE_*` constants; cleanup =
  `cleanup_upper_whiteouts` pre-rmdir sweep (pure-upper arm) +
  `is_whiteout_inode` predicate extraction + clear-empty preserved for the
  lower-backed arm (displaced-dir leg routed through the sweep seam,
  best-effort). Next step: main-agent Creator pass slicing when the user
  authorizes implementation.

- **`pass_40_wave7_impure_cleanup_20260808`**
  - **Kind**: Creator Pass (High risk; bounded cross-meso implementation of
    the ACCEPTED `wave7_impure_cleanup_design` contract; user-directed
    dispatch via the WSL codex CLI, main-agent acceptance).
  - **Parent**: `N/A` — cross-meso (precedent: `pass_28_wave7_logic_bug_repair`);
    per-objective owners preserved (O1: Meso 04/05/06; O2: Meso 06 + Meso 02
    predicate + two Meso 03 widenings).
  - **Covered Micro-Features**: `P2-03`, `P1-33`, `P1-01..P1-07`,
    `P1-25`, `P1-27`, `P1-28`, `P1-31`, `P1-36`.
  - **Write-set**: `metadata_security/xattr.rs`, `copyup/promote.rs`,
    `dir/{mod.rs,rename.rs,whiteout.rs,remove.rs}`,
    `projection/entry.rs`, `readdir_index.rs` + Creator report
    `components/wave7_impure_cleanup_design/pass_40_wave7_impure_cleanup_creator.md`.
  - **Execution shape**: command-free (compile withheld); frozen surface per
    spec §1.4/§2.4; Bug B out of scope; main agent performs exact-diff
    acceptance + target-specific `cargo check` after the Creator reports.
  - **Packet**:
    `subagent-tasks/wave7_impure_cleanup_design/task_creator_wave7_impure_cleanup_20260808_dispatch.md`.
  - **Acceptance (2026-08-08): ACCEPTED with recorded deviations.** Creator
    (via WSL codex CLI, inherited `~/.codex`) implemented the frozen surface:
    `IMPURE_*` constants + `OverlayXattrPolicy::{has,set,clear}_impure_marker`
    + `OverlayInode::refresh_impure_marker` (xattr.rs), T1-T4 triggers
    (promote/mod/link/rename/whiteout), C1/C2 best-effort refreshes,
    `is_whiteout_inode` extraction + delegation (entry.rs),
    `cleanup_upper_whiteouts` sweep seam (whiteout.rs), branch-A/branch-E
    sweep call sites, clear-empty leg routed through the seam, two index
    widenings (+ recorded third `ReaddirIndex::entries` widening),
    projection/mod.rs one-line re-export (recorded incidental edit). Report
    `components/wave7_impure_cleanup_design/pass_40_wave7_impure_cleanup_creator.md`
    with full census + 6 recorded deviations (all accepted; behavior-
    preserving or forced by the frozen location). Main-agent verification:
  `cargo check -p aster-kernel --target x86_64-unknown-none` exit 0 (only
    pre-existing `MountPolicy::uuid_mode` warning); `cargo fmt --check` clean
    after the main-agent mechanical `cargo fmt` of the changed files. Bug B
    untouched. Next step: user-authorized `overlay/031` + `overlay/038`
    validation re-run (Checker lane; ls3 expected to remain a Bug B
    dependency failure).
  - **Validation (2026-08-08, `pass_41_wave7_impure_cleanup_checker.md`):
    `overlay/038` PASS** (impure set + clear + filter assertions all pass);
    **`overlay/031` in-scope objectives VERIFIED** — both `ENOTEMPTY`
    failures fixed by the sweep, rm3 lower-backed publish correct; the single
    remaining diff line is ls3, the recorded out-of-scope Bug B dependency
    (expected to remain FAIL until the base-view coherence fix). No repair
    batch for pass_40. Evidence under
    `components/wave7-xfstests-sequencing/run_evidence/{overlay031,overlay038}/impure_cleanup_20260808/`.

- **`pass_42_wave7_bug_b_path_repair`**
  - **Kind**: Creator Pass (High risk; bounded cross-meso implementation of
    the ACCEPTED `wave7_bug_b_path_design` contract —
    `components/wave7_bug_b_path_design/wave7_bug_b_path_designer_spec.md` +
    `_designer_validation.md`; user-directed 3-phase execution).
  - **Parent**: `N/A` — cross-meso (precedent: `pass_40_wave7_impure_cleanup`);
    per-objective owners preserved (meso 01 anchors/claims, meso 02
    `RealObject` carrier, meso 04 workdir/copy-up leg, meso 06 dir arms,
    meso 03 readdir adjacency).
  - **Covered Micro-Features**: `P1-26`, `P1-36`, `P1-01..P1-07`, `P1-34`,
    `P0-14`, `P1-31`, `P1-33`. Adjacency surface (touched call sites, NOT
    claimed): `P1-22`, `P1-23`, `P1-25`, `P1-27`, `P1-28`, `P1-29`,
    `P1-30` (recorded in the Designer spec §0.1).
  - **Phase slicing (user-directed 2026-08-09; one pass number, three serial
    phases for bounded acceptance; command-free, no per-phase compile):**
    - **Phase A — 载体/锚点/查找路由** (foundation seam; no standalone
      micro-feature claim, precedent `pass_03_shared_carrier_seams`).
      Write-set: `mount/layers.rs`, `mount/claims.rs`,
      `projection/entry.rs`, `projection/inode.rs`, `dir/mod.rs`
      (add `upper_parent_path()` only).
    - **Phase B — workdir temp 载体 + copy-up/link 腿**.
      Write-set: `copyup/workdir.rs`, `copyup/promote.rs`, `dir/link.rs`;
      mechanical `create_workdir_temp` call-site param swaps only at
      `dir/create.rs`, `dir/whiteout.rs`, `dir/remove.rs`.
    - **Phase C — dir 系语义扫尾 + 旧接口删除**.
      Write-set: `dir/create.rs`, `dir/whiteout.rs`, `dir/remove.rs`,
      `dir/rename.rs`, `dir/mod.rs` (link_impl + deletion of `upper_parent`),
      `copyup/workdir.rs` + `copyup/promote.rs` (deletion of the now-dead
      `workdir_root` inode accessors), `mount/claims.rs` (removal of the
      Phase A `#[expect(dead_code)]` on `workdir_workspace_path`).
  - **Execution shape**: one Creator dispatch per phase via the V2 lane
    (new User Dispatch Turn + spawn `fork_turns="1"`; Phase B/C carry the
    Continuation / Parent Task pointer). Main-agent structural diff
    acceptance per phase; NO compile during phases. After Phase C, one
    target-specific `cargo check -p aster-kernel --target
    x86_64-unknown-none`; the main agent fixes only mechanical compile
    errors (imports/unused/visibility mechanicals per Wave5 precedent);
    any missing-surface or semantic error routes back to the Creator as a
    new repair dispatch turn.
  - **Git discipline (user-directed)**: Phase A creates a new commit; Phase
    B and Phase C each `--amend` that same commit (stage only the phase's
    production `.rs` write-set), so each phase's `git diff HEAD` shows only
    that phase and the final history is one new commit for the whole pass.
    `.agents` records stay uncommitted unless the user directs otherwise.
  - **Execution / compile gate (2026-08-09)**: Phases A/B/C all landed and
    were structurally accepted; the working tree is amended into one commit
    (`4830cd007`, 12 files, +582/-314). The first container compile
    (`cargo check -p aster-kernel --target x86_64-unknown-none`) FAILED with
    exactly 8 `E0599` errors — `Dentry::lookup_child` is unreachable from
    overlayfs (method is on `DirDentry`; its producer `as_dir_dentry_or_err`
    and the `DirDentry` struct are `pub(super)`; `Dentry::new` is private).
    Root cause + probe evidence:
    `components/wave7_bug_b_path_design/compile_failure_lookup_child_20260809.md`.
    A bounded Designer revision (`task_designer_wave7_bug_b_lookup_revision_20260809`)
    is dispatched to confirm the reachable replacement
    (`PathResolver::lookup_at_path`, resolver.rs:555) across the 8 sites,
    the resolver-acquisition lock discipline, and the MAY_EXEC verdict.
    The pass is NOT compile-accepted until the revision lands and the 8
    sites are replaced; no per-phase compile was run before this gate.
  - **Boundaries**: no VFS interface change; no static lock-topology or
    Meso-owner change; no ktest surface; no `legacy_fs.rs`; no Creator
    pass slicing or `.agents` status edits; no build/test commands in
    Creator phases. Frozen surface per the Designer spec §1/§4 and the
    per-phase rows of spec §5.7; every deviation must be recorded in the
    Creator report.
  - **Validation gate (2026-08-10, user-authorized; OUTCOME B — actionable
    repair batch):** `task_checker_wave7_cache_consistency_20260810` ran the
    12-case schedulable regression (fresh 8 GiB ext2 images, one QEMU per
    case, per-case evidence under
    `components/wave7_cache_consistency_design/run_evidence/pass43_regression_20260810/`;
    receipt `components/wave7_cache_consistency_design/pass_43_wave7_cache_consistency_checker.md`).
    **11 PASS / 1 FAIL / 0 NOTRUN / 0 HANG** — PASS: 002 003 006 007 010 011
    014 024 031 (whole-case, ls3 included — Bug B NOT re-opened) 038 077;
    **FAIL: overlay/012** — expected `rm: cannot remove 'SCRATCH_MNT/test':
    Stale file handle` (ESTALE), got `Is a directory` (EISDIR). Attribution:
    pass_43 **Change 1** (`lookup_binding` always-scan + verify-then-serve,
    projection/mod.rs L88/94) — the unconditional fresh scan re-derives the
    lower `test` directory instead of serving the cached Positive(upper file),
    so `remove_target` hits the EISDIR defensive gate (dir/remove.rs L191-194)
    and the `translate_stale_upper_enoent` ESTALE arm (dir/remove.rs L518) is
    unreachable. No panic/oops/warn; no infrastructure failure. **Repair
    batch (Checker §4):** bounded follow-up Creator pass with Designer
    sign-off — distinguish in `lookup_binding`'s fresh-truth derivation
    between true lower fall-back and stale-upper (a Positive upper binding was
    published for `(parent_id,name)` and the upper object was deleted behind
    the overlay with no whiteout residue); the stale-upper case must route
    `remove_target` through `translate_stale_upper_enoent` → ESTALE. Fix then
    single-run `overlay/012` (fresh 8 GiB) to confirm ESTALE + no warn/oops,
    then re-run the full 12-case table to confirm the other 11 stay green.
    No ktest, no VFS change, no Bug B routing. pass_43 is NOT gate-accepted
    until the repair lands and the re-run passes.
  - **Validation (later, user-authorized)**: meso-integration Checker per
    `wave7_bug_b_path_designer_validation.md` — `overlay/031` direct (ls3 +
    cross-run residue; rm1/2/3 regression guards) + adjacency
    `010/024/012/003/006/011/029/077/038`; `020/041/013` out of scope.

- **`pass_43_wave7_cache_consistency`**
  - **Kind**: Creator Pass (High risk; bounded cross-meso implementation of
    the ACCEPTED `wave7_cache_consistency_design` contract —
    `components/wave7_cache_consistency_design/wave7_cache_consistency_designer_spec.md`
    + `_designer_validation.md`; user-directed 3-phase execution 2026-08-10).
  - **Parent**: `N/A` — cross-meso (precedent:
    `pass_42_wave7_bug_b_path_repair`); per-change owners preserved: meso 02
    (`projection/{inode,inode_cache,binding_cache,mod}.rs`), meso 04
    (`copyup/promote.rs` — one-line `replace_facts` parameter), meso 03
    (`readdir_index.rs` — doc-only Change 5).
  - **Covered Micro-Features**: `P0-08`, `P0-09`, `P0-10`, `P0-11`,
    `P0-16` (Direct); `P0-14`, `P1-31` (Supporting, unchanged seams);
    `P1-36`, `P1-07` (Adjacency, untouched) — reconciled in Designer spec
    §0.1. No micro ID is invented.
  - **Phase slicing (user-directed 2026-08-10; one pass number, three serial
    phases for bounded acceptance; command-free, no per-phase compile):**
    - **Phase A — 身份判定 / memo 验证原语** (foundation primitive seam; no
      standalone micro-feature claim, precedent `pass_03_shared_carrier_seams`
      / `pass_42` Phase A). Write-set: `projection/inode.rs`
      (`OverlayObjectFacts::same_visible_identity`,
      `OverlayObjectFacts::contains_real_inode`),
      `projection/binding_cache.rs` (`Binding::matches_truth`,
      `NegativeBinding::is_same_negative`,
      `BindingCache::invalidate_parent`). All five new methods carry
      `#[expect(dead_code)]` pending their wiring phases; NO call-site or
      control-flow change.
    - **Phase B — 载体缓存校验与失效接线**. Write-set:
      `projection/inode_cache.rs` (`get_or_create` 3-arg same-object
      validation + stale replacement; `alias_key` 4-arg F1/F2 displacement
      refinement), `projection/inode.rs` (`replace_facts` gains
      `new_visible_source: &RealObject` + directory-gated
      `invalidate_parent` call; `new_root` call site passes `|_| true`),
      `projection/mod.rs` (`project_inode` call site passes the
      `contains_real_inode` closure), `copyup/promote.rs` (one-line
      `carrier.replace_facts(new_facts, &upper_real)` at promote.rs:527).
      Remove `#[expect(dead_code)]` from `contains_real_inode` and
      `invalidate_parent` (wired here); keep on `same_visible_identity` /
      `matches_truth` / `is_same_negative`.
    - **Phase C — lookup_binding memo 化 + readdir 边界文档**.
      Write-set: `projection/mod.rs` (`lookup_binding` rewritten to always
      scan `lookup_in_layers` then serve only on `matches_truth`), 
      `projection/binding_cache.rs` (remove the remaining
      `#[expect(dead_code)]` — now wired), `readdir_index.rs` (doc-only
      Change 5 paragraph on `ReaddirIndexValidity::Valid`).
  - **Execution shape**: one Creator dispatch per phase via the Direct Spawn
    Lane (PROTOCOL §1.3 preferred, platform-verified; continuation rounds are
    new spawns carrying the Continuation / Parent Task pointer). Main-agent
    structural diff acceptance per phase; NO compile during phases. After
    Phase C, one target-specific `cargo check -p aster-kernel --target
    x86_64-unknown-none`; the main agent fixes only mechanical compile errors
    (imports/unused/visibility mechanicals per Wave5 precedent); any
    missing-surface or semantic error routes back to the Creator as a new
    repair dispatch.
  - **Git discipline (user-directed)**: Phase A creates a new commit; Phase
    B and Phase C each `--amend` that same commit (stage only the phase's
    production `.rs` write-set), so each phase's `git diff HEAD` shows only
    that phase and the final history is one new commit for the whole pass.
    `.agents` records stay uncommitted.
  - **Boundaries**: zero VFS interface change; no new lock domain / no
    static lock-topology change (Designer spec §3.4 acyclicity: cache locks
    are leaf locks, never nested with each other or with `DIR`/`CUL`);
    no ktest surface; no `legacy_fs.rs`; no Creator pass slicing or `.agents`
    status edits; no build/test commands in Creator phases. Frozen surface
    per Designer spec §2 (Hoare cases + frozen Rust representation) / §4
    (signatures / owner placement); every deviation recorded in the Creator
    report.
  - **Execution / compile gate (2026-08-10)**: Phases A/B/C all landed and
    were structurally accepted per phase; the working tree is amended into one
    commit (`7d6b5c960`, 6 files, +284/-75). Container compile
    (`cargo check -p aster-kernel --target x86_64-unknown-none`) PASSED with
    zero errors. Compile-gate warning cleanup (main-agent mechanicals per
    Wave5/pass_42 precedent): removed the four now-obsolete
    `#[expect(dead_code, reason = ...)]` attributes on
    `HiddenEvidence::{layer_index, real_inode}` and the
    `NegativeBinding::{HiddenByWhiteout,HiddenByOpaque}` payloads (both
    fields/variants are genuinely read by `is_same_negative` after Phase C),
    and narrowed `Binding::matches_truth` visibility to `pub(super)`
    (its `&LayerLookup` parameter is projection-internal — the frozen
    `pub(in crate::fs::fs_impls::overlayfs)` widened the interface past the
    parameter type; `pub(super)` matches the spec §4.4 allowance for
    projection-internal methods). Remaining warning is the pre-existing
    `MountPolicy::uuid_mode` (recorded; not part of this pass). The pass is
    compile-accepted.
  - **Validation gate (2026-08-10, user-authorized; OUTCOME B — actionable
    repair batch):** `task_checker_wave7_cache_consistency_20260810` ran the
    12-case schedulable regression (fresh 8 GiB ext2 images, one QEMU per
    case, per-case evidence under
    `components/wave7_cache_consistency_design/run_evidence/pass43_regression_20260810/`;
    receipt `components/wave7_cache_consistency_design/pass_43_wave7_cache_consistency_checker.md`).
    **11 PASS / 1 FAIL / 0 NOTRUN / 0 HANG** — PASS: 002 003 006 007 010 011
    014 024 031 (whole-case, ls3 included — Bug B NOT re-opened) 038 077;
    **FAIL: overlay/012** — expected `rm: cannot remove 'SCRATCH_MNT/test':
    Stale file handle` (ESTALE), got `Is a directory` (EISDIR). Attribution:
    pass_43 **Change 1** (`lookup_binding` always-scan + verify-then-serve,
    projection/mod.rs L88/94) — the unconditional fresh scan re-derives the
    lower `test` directory instead of serving the cached Positive(upper file),
    so `remove_target` hits the EISDIR defensive gate (dir/remove.rs L191-194)
    and the `translate_stale_upper_enoent` ESTALE arm (dir/remove.rs L518) is
    unreachable. No panic/oops/warn; no infrastructure failure. **Repair
    batch (Checker §4):** bounded follow-up Creator pass with Designer
    sign-off — distinguish in `lookup_binding`'s fresh-truth derivation
    between true lower fall-back and stale-upper (a Positive upper binding was
    published for `(parent_id,name)` and the upper object was deleted behind
    the overlay with no whiteout residue); the stale-upper case must route
    `remove_target` through `translate_stale_upper_enoent` → ESTALE. Fix then
    single-run `overlay/012` (fresh 8 GiB) to confirm ESTALE + no warn/oops,
    then re-run the full 12-case table to confirm the other 11 stay green.
    No ktest, no VFS change, no Bug B routing. pass_43 is NOT gate-accepted
    until the repair lands and the re-run passes.
  - **Validation (later, user-authorized)**: meso-integration Checker per
    `wave7_cache_consistency_designer_validation.md` §2 — **schedulable
    regression batch** (fresh images, one QEMU per case; expected `PASS`
    incl. 031 whole-case — Bug B ls3 fixed 2026-08-09, see handoff §8.5):
    required `002/003/010/012/014/024/031/038/077`; recommended cheap additions
    `006/007/011` (validation §1 mapped, `PASS`, same surfaces). **Mapped
    but NOT schedulable / not-run** (recorded in the `not-run` column with
    reason, NOT run): `001` (H ≥8 GiB; known underlying block defect —
    combined only if user authorizes), `004` (fsgqa env gate), `005`
    (loop+XFS harness gap), `017` (redirect_dir P2-02 deferred), `018`/`037`
    (index=on deferred), `020` (userns capability gate), `021` (F2
    out-of-scope + ≥16 GiB), `061` (closed VM/page-cache defect); `019`
    (fsstress) only as an optional user-authorized lock-order observation.

- **`pass_44_wave7_cache_consistency_012_repair`**
  - **Kind**: Creator Pass (High risk; bounded repair of the pass_43 Change 1
    stale-upper regression, per the Checker OUTCOME B repair batch + the
    ACCEPTED Designer 会签
    `components/wave7_cache_consistency_design/wave7_cache_consistency_012_repair_designer_spec.md`
    + `_designer_validation.md`; task
    `task_designer_wave7_cache_consistency_012_repair_20260810`).
  - **Parent**: `N/A` — bounded cross-meso repair (precedent: pass_40/pass_42
    repair passes): meso 02 (`projection/binding_cache.rs`
    `Binding::is_stale_upper`, `projection/mod.rs` `LookupOutcome` +
    `lookup_binding` return), meso 06 (`dir/remove.rs` `remove_target` step-1
    ESTALE routing); 7 mechanical `.binding` call-site adaptations in
    `projection/inode.rs`, `readdir_index.rs`, `dir/create.rs`,
    `dir/rename.rs`, `dir/mod.rs`.
  - **Continuation / Parent Task**: pass_43_wave7_cache_consistency
    meso-integration Checker gate (2026-08-10 OUTCOME B; receipt
    `components/wave7_cache_consistency_design/pass_43_wave7_cache_consistency_checker.md`
    §4 — verbatim diagnostics preserved in the 012_repair spec §0).
  - **Covered Micro-Features**: `P0-08`, `P0-11` (the pass_43 direct micros
    exercised by `overlay/012`'s stale-upper sequence). No new micro.
  - **Frozen surface (ACCEPTED Designer 会签)**: `Binding::is_stale_upper`
    (pub(super), Rule A; stale ⇔ cached positive upper-backed binding whose
    fresh truth no longer contains that upper entry with no whiteout left);
    `LookupOutcome { binding, is_stale_upper }` (Rule D publication-result
    carrier); `lookup_binding -> Result<LookupOutcome>` (probe derives the
    signal when `matches_truth` fails; rebuild + publish + copy-up hook
    unchanged); `remove_target` step 1 consumes `is_stale_upper` BEFORE the
    rmdir-emptiness and EISDIR gates and routes `Err(ESTALE)` via the
    unchanged `translate_stale_upper_enoent`; 7 mechanical `.binding`
    adaptations; zero VFS change, zero new lock domain, zero new error
    variant, zero ktest.
  - **Execution shape**: one Creator dispatch (Direct Spawn Lane, command-
    free) → main-agent structural diff acceptance → one target-specific
    `cargo check -p aster-kernel --target x86_64-unknown-none` (main agent
    fixes only mechanical errors) → amend into a new commit → Checker
    re-validation per `wave7_cache_consistency_012_repair_designer_validation.md`:
    step (i) single-case `overlay/012` (fresh 8 GiB; expect
    `Stale file handle` + no warn/oops), step (ii) full 12-case pass_43 table
    (11 stay green, 012 becomes PASS; 0 NOTRUN, 0 HANG). No Bug B routing
    (031 passed whole-case on the gate).
  - **Execution / compile gate (2026-08-10)**: Creator ACCEPTED
    (`pass_44_wave7_cache_consistency_012_repair_creator.md`; diff verified
    against the frozen 会签 surface: `is_stale_upper` verbatim,
    `LookupOutcome` + probe delta, `remove_target` step-1 ESTALE routing, 7
    mechanical `.binding` sites; one mechanical double-blank-line fixed).
    Container `cargo check -p aster-kernel --target x86_64-unknown-none`
    PASSED (0 errors; only the pre-existing `uuid_mode` warning). Committed
    `cd29d9c17` (8 files, +112/-26).
  - **Re-validation (2026-08-10, ACCEPTED — OUTCOME A)**: `task_checker_wave7_cache_consistency_012_repair_20260810`
    (agent Peirce; kernel `cd29d9c17`). Step (i) `overlay/012` single-case
    **PASS** — `Ran: overlay/012` / `Passed all 1 tests` / `All conformance
    tests passed`, zero mismatch, guest golden `012.out` verbatim
    `rm: cannot remove 'SCRATCH_MNT/test': Stale file handle` (ESTALE);
    serial warn/oops scan 0 hits. Step (ii) full 12-case table **12/12 PASS,
    0 NOTRUN, 0 HANG** (all rc=0; `RESULTS_step2.txt`); `overlay/031` whole
    case incl. `ls3`, no Bug B routing. Evidence:
    `components/wave7_cache_consistency_design/run_evidence/pass44_012_repair_20260810/`
    (13 runs); receipt
    `pass_44_wave7_cache_consistency_012_repair_checker.md`. **pass_43 +
    pass_44 gate-accepted**; §8 unified ledger 012 restored to green (20/43).
  - **overlay/021 targeted retry (2026-08-10, user-directed; evidence run):**
    re-ran `overlay/021` (16 GiB images, commit `cd29d9c17`) to test whether
    the pass_43/44 cache/identity fixes changed the concurrent copy-up
    outcome. Result: FAIL at the SAME seeding/precondition stage with the
    SAME symptom as 2026-08-09 (4 lower-arena globs `*0/*4/*8/*b` miss +
    `find: 'p2'/'p3'`); no panic/hang; no later-stage concurrency evidence.
    Attribution unchanged (`harness/前置`, non-overlayfs); NOT a pass_43/44
    regression. Receipt `components/wave7_cache_consistency_design/pass_43_021_retry_checker.md`;
    evidence `run_evidence/overlay021_retry_20260810/`. **Root cause pinned
    (2026-08-10 guest fsstress smoke, user-directed):** fsstress exits at
    `io_setup` (aio, syscall 206) → ENOSYS (no Asterinas aio handler;
    `kernel/core/src/syscall/mod.rs` default branch) BEFORE any file op → 0 files
    seeded → 021 globs empty. Attribution refined to `前置/内核 syscall 缺口`,
    non-overlayfs; routing options (a) implement io_setup/io_destroy
    (out-of-scope, separate authorization), (b) repackage AIO-less fsstress,
    (c) alternate seeder. Receipt
    `pass_43_021_fsstress_smoke_checker.md`; evidence
    `run_evidence/fsstress_smoke_20260810/`; harness reverted (git clean).
  - **Execution / compile gate (2026-08-10)**: Creator ACCEPTED
    (`components/nested_mount_claim_lifetime_design/pass_45_nested_mount_claim_lifetime_creator.md`;
    diff verified against the frozen 会签 surface: `RealPath` carrier +
    `from_path`/`upgrade`/`inode`, `root_path`/`real_path` type deltas,
    `lookup_in_layers` storage, 7 mechanical adaptations, zero VFS/syscall/
    legacy touches; one incidental visibility seam `mount/mod.rs`
    `pub(in crate::fs::fs_impls::overlayfs) use layers::RealPath;` mirroring
    the existing `XinoMode` re-export). Container
    `cargo check -p aster-kernel --target x86_64-unknown-none` PASSED
    (0 errors; only the pre-existing `uuid_mode` warning). Committed
    `c92c21b67` (12 files incl. PASS_SLICING/handoff, +251/-49).
  - **Validation (2026-08-10, ACCEPTED — OUTCOME A)**: `task_checker_nested_mount_claim_lifetime_20260810`
    — **scoped to `overlay/029` single-case only** (user-directed: the full
    20-case regression matrix is deferred until after wave8; the Designer
    validation contract's step (ii) is not run in this gate). Fresh 8 GiB
    images, whole-case PASS expected, zero EBUSY, zero warn/oops, post-test
    remount succeeds. Result (agent Kierkegaard; kernel `c92c21b67`):
    `overlay/029` whole-case **PASS** — `Ran: overlay/029` / `Passed all 1
    tests` / `All conformance tests passed.` (exit 0); no `already mounted or
    mount point busy.`, no `try_claim`/EBUSY trace, zero kernel warn/oops/
    panic; fresh 8 GiB TEST/SCRATCH image proof in `make_run_kernel.out` (two
    `Creating filesystem with 2097152 4k blocks` lines); unmount→remount
    invariant satisfied. Evidence
    `components/nested_mount_claim_lifetime_design/run_evidence/overlay029_pass45_20260810/`;
    receipt `pass_45_nested_mount_claim_lifetime_checker.md`. **pass_45
    gate-accepted**; the full 20-case regression remains deferred to after
    wave8 (recorded deferral, not a coverage gap).
  - **Boundaries**: no VFS interface change; no static lock-topology change;
    no ktest; no `legacy_fs.rs`; no `.agents` status edits by the Creator;
    no build/test commands in the Creator phase.

- **`pass_45_nested_mount_claim_lifetime`**
  - **Kind**: Creator Pass (High risk; bounded repair of the overlay/029
    nested-mount claim-lifetime / self-reference-ring regression, per the
    ACCEPTED Designer 会签
    `components/nested_mount_claim_lifetime_design/nested_mount_claim_lifetime_designer_spec.md`
    + `_designer_validation.md`; task
    `task_designer_nested_mount_claim_lifetime_20260810`; user-confirmed
    scheme 2026-08-10: B1-local).
  - **Parent**: `N/A` — bounded cross-meso repair (precedent: pass_40/pass_42
    repair passes): meso 01 (`mount/layers.rs` — new `RealPath` carrier +
    `OverlayLayer.root_path`), meso 02 (`projection/entry.rs` —
    `RealObject.real_path`, `projection/inode.rs` — `new_root`); 7 mechanical
    call-site adaptations (`dir/mod.rs`, `dir/link.rs`, `dir/remove.rs`,
    `dir/rename.rs`, `copyup/promote.rs`, `dir/create.rs` ×2).
  - **Continuation / Parent Task**: ACCEPTED Designer 会签
    `task_designer_nested_mount_claim_lifetime_20260810` (2026-08-10).
  - **Covered Micro-Features**: `P1-35` (direct — claim release lifetime),
    `P0-02` (layer-stack anchor), `P0-16` (per-inode real-path carrier).
    No new micro.
  - **Frozen surface (ACCEPTED Designer 会签)**: `RealPath` (new, Rule D,
    `mount/layers.rs`; `Weak<Mount>` + `Arc<Dentry>` + `Arc<dyn Inode>`;
    `from_path` / `upgrade -> Result<Path>` / `inode`); `OverlayLayer.root_path:
    Path -> RealPath`; `RealObject.real_path: Option<Path> -> Option<RealPath>`;
    `RealObject::with_path` param delta + `real_path() -> Result<Path>` owned
    return delta; 9 mechanical call-site adaptations (spec §5); zero VFS
    change, zero new lock domain, zero new error variant, zero ktest.
  - **Execution shape**: one Creator dispatch (no-fork, command-free) →
    main-agent structural diff acceptance → one target-specific
    `cargo check -p aster-kernel --target x86_64-unknown-none` (main agent
    fixes only mechanical errors) → amend into a new commit → Checker
    validation **scoped to `overlay/029` single-case only** (user-directed
    2026-08-10: the full 20-case regression is deferred until after wave8;
    the Designer validation contract's step (ii) is re-scoped accordingly).
  - **Boundaries**: no VFS interface change; no static lock-topology change;
    no ktest; no `legacy_fs.rs`; no `.agents` status edits by the Creator;
    no build/test commands in the Creator phase.

- **`pass_01_fs_creation_ctx_repair`**
  - **Kind**: Creator Pass (bounded cross-meso API-repair；upstream `FsCreationCtx` 移除
    `task_ctx` 字段后的编译修复；task `task_creator_fs_creation_ctx_repair_20260813`；
    ACCEPTED Designer 调研 `components/fs_creation_ctx_research_20260813/fs_creation_ctx_designer_research_20260813.md` §4
    + `_validation_note.md`）。
  - **Parent**: `N/A` — bounded cross-meso repair（precedent: pass_45）：meso 01 mount 树
    （`build.rs`/`layers.rs`/`claims.rs`/`policy.rs`/`mod.rs`）+ VFS `registry.rs` 删除残留访问器。
  - **Continuation / Parent Task**: ACCEPTED `task_designer_fs_creation_ctx_research_20260813`
    （方案 1：overlayfs 内部用 `Task::current().as_posix_thread()` 拿挂载线程；VFS 零改动，
    `registry.rs` 回 upstream `76dac6f55` 原状；方案 2 Linux fs_context 回放原理可行但被方案 1 支配）。
  - **Covered Micro-Features**: 无新 micro（纯 API-repair；语义不变：同一挂载线程凭证快照 +
    同一 resolver 路径解析，与 Linux `fc->cred`/`kern_path(AT_FDCWD)` 语义同型）。
  - **Frozen surface (ACCEPTED Designer 调研)**: `with_current_posix_thread<T>(operation_fn:
    impl FnOnce(&PosixThread) -> Result<T>) -> Result<T>`（`mount/mod.rs`，`pub(super)`，
    两个 `EINVAL` fail-closed 分支，无 unwrap/expect）；`resolve_root_path(raw_path)` /
    `resolve_parts(raw_path)` / `assemble(upper_dir, lower_dirs, is_forced_read_only)` /
    `verify_inode_instance_stability(raw_path, pinned_inode)` 去 ctx 参数；`build.rs` 凭证快照
    走 `with_current_posix_thread` + `credentials_dup()`（4a 位置与 drop 顺序不变）；`policy.rs`
    仅注释；`registry.rs` 删 `task_ctx()` 访问器（−5 行，回 upstream 原状）。零 ktest，零 unsafe，
    `legacy_fs.rs` 未动。
  - **Execution shape**: Designer 调研（V1 架构）→ 1 个 Creator dispatch（V1 架构，command-free）
    → main-agent 结构 diff 验收 → container `cargo check -p asterinas --target x86_64-unknown-none`
    **PASSED**（main agent 亲自执行，9.91s，0 errors）→ 验收点 1-7 全过（registry diff 为空；
    无 `.task_ctx()` 调用；签名一致；无 unwrap/expect/unsafe/ktest/legacy 改动）→ 提交。
  - **Boundaries**: 无新锁/锁序变更；无 VFS 增量（`registry.rs` 与 upstream 逐字节一致）；
    Creator 无构建命令；不改 `.agents` 状态文件；运行时行为不变，xfstests 回归为可选（未调度）。

- **`wave9_comments_abflex_audit_20260815`**
  - **Kind**: bounded Reviewer wave（comments A/B-flexibility full-tree audit；
    read-only；6 个并行 Reviewer；task group
    `wave9-comments-abflex-audit-20260815`）。
  - **Parent**: `N/A` — cross-meso full-tree comment review（不改变任何
    meso/micro 归属；无 covered micro）。
  - **User directive (2026-08-15)**: 担忧此前执行审查的模型难以处理灵活性
    判据，指示对 overlayfs 全树现存所有 comments 按 A/B 两档灵活性原则
    （A：N1/N2/C2/C3/P2/N10/N11；B：N9/N5/C4/C1/P10/P5/P9′）再做一次全面
    诚实只读 Review。最多 6 个 subagent（mount / projection / dir / copyup /
    security / top_readdir），不执行清理。
  - **Scope write-set**: 每个 Reviewer 仅可写其自身报告
    `components/wave9-comments-abflex-audit-20260815/<task_id>_audit.md`；
    无任何 `.rs` 编辑。
  - **Criteria**: `subagent-tasks/wave9-comments-abflex-audit-20260815/
    wave9_comments_abflex_audit_CRITERIA.md`（仅 A/B 14 项为正式判据；
    C/D 档不报；PR3708 A/B 档 finding 复核后按 `REMAINING` 再报；
    user 裁决 M-AUDIT-1/2/3、§4.2 延迟、fullaudit cleanup7 回潮复核）。
  - **Boundaries**: read-only；不运行 cargo/rustfmt/clippy/git 写命令；
    不 spawn/message；`legacy_fs.rs`/`.agents/` 不审；不审查代码逻辑；
    P9′/P5 代码级项仅标 `adjudicate`。
  - **Result (2026-08-15, ACCEPTED)**: 6/6 报告收齐并由 main agent 逐条验收；
    13 findings（N9×6/N5×4/P5×1/P2×1/C1×1；REMAINING 10=PR3708 A/B 档、
    NEW 3；0 REGRESSION）；无 `.rs` 改动；cleanup 不授权，执行待 user 指令。

- **`wave9_comments_abflex_exec_20260815`**
  - **Kind**: bounded two-lane execution wave（user-directed 2026-08-15）。
  - **Lane 1（mechanical）**: 1 个 **deepseek-v4-flash Creator** 按 packet 精确 old/new
    执行 ABFLEX 机械档 3 条（M-AB-3 删 layers.rs 锁注释块；CP-AB-1 删 coordination.rs
    自指路径；D-AB-1 重写 dir/link.rs 模块 doc 补 DIR lock contract）。
    write-set: 3 个 `.rs` 指定注释 + 1 收据。
  - **Lane 2（flexible debate）**: 2 个 **deepseek-v4-pro** 按
    `flexible_plan_debate_BRIEF.md` 对 ABFLEX 灵活档 10 条做 A 提案 → B 批驳 → A 终稿
    三轮辩论；产出 `flexible_plan_final.md`。**不修改 `.rs`**。
  - **Parent**: `N/A` — comment-only continuation of wave9 ABFLEX audit.
  - **Boundaries**: 机械档只允许 packet §2 三处注释改动，禁代码改动/编译；
    灵活档只允许三个 plan 文件；不执行 flexible 修改（待 user 批准）。
  - **Result (2026-08-15, ACCEPTED)**: Flash 机械档 3/3 精确执行（0 代码改动）；
    双 Pro 辩论完成，终稿 10/10 覆盖并经 main agent 验收注记（含 4 处自检计数校正）；
    灵活档未实施，执行待 user 指令（M-AB-1 口径差异待批准）。

- **`wave9_comments_abflex_flexexec_20260815`**
  - **Kind**: bounded Creator wave（user-approved 2026-08-15 flexible plan execution；
    comment-only）。
  - **Parent**: `N/A` — continuation of `wave9_comments_abflex_exec_20260815` Lane 2.
  - **Scope slicing**: 4 个 deepseek-v4-flash Creator 并行、按文件 write-disjoint：
    mount（M-AB-1/M-AB-2，options.rs）、projection（PRJ-AB-1/2/3，mod.rs+entry.rs）、
    security（S-AB-1/2，metadata.rs+xattr.rs）、top_readdir（TR-AB-1/2/3，readdir_index.rs）。
  - **Execution basis**: `flexible_plan_exec_SPEC.md`（main-agent 核准 exact old/new，
    来自双 Pro 终稿 `flexible_plan_final.md`）；各 packet 只授权对应 SPEC 节的注释替换。
  - **Boundaries**: 只改 SPEC 列出的注释；禁代码改动/格式化/编译；不碰其它文件。
  - **Result (2026-08-15, ACCEPTED)**: 4/4 Flash Creator 收据在册；10/10 替换与
    `flexible_plan_exec_SPEC.md` 逐字一致；9 个 `.rs` diff 仅注释行、0 代码行改动；
    未编译、未提交。

- **`wave9_topdocs_review_20260815`**
  - **Kind**: bounded Reviewer wave（user-directed 2026-08-15；1 个 Reviewer、只读）。
  - **Parent**: `N/A` — full-tree `//!` module-doc quality review（不含 `///`/行内 `//`）。
  - **Basis**: book guides：maintainability `comments.md`、rust-specific
    `comments.md` + `crates-and-modules.md`（module-docs）、documentation
    `general-style.md`；handoff §4.1 为背景。
  - **Scope**: overlayfs 全部 32 个 `.rs` 的 `//!` doc（legacy_fs.rs/.agents 除外）。
  - **Boundaries**: read-only；唯一写产物 = 1 份审计报告
    `components/wave9-topdocs-review-20260815/`；不执行修复。
  - **Result (2026-08-15, ACCEPTED)**: 报告 11 findings（module-docs 2 NEW；P3 7
    REMAINING；no-impl-in-docs 1；explain-why 冗余 1）；main agent 逐条锚定核实；
    修复未授权，执行待 user 指令。

- **`wave9_topdocs_fix_20260815`**
  - **Kind**: bounded Creator pass（user-directed 2026-08-15；comment-only）。
  - **Parent**: `N/A` — continuation of `wave9_topdocs_review_20260815`.
  - **Scope**: 8 处 exact old→new——T1 顶层 `overlayfs/mod.rs` 补 `//!` doc
    （参考 ext2/exfat 写法）；P3-1..7 执行 TOPDOCS F3–F9（去跨文件路径引用）。
  - **Basis**: `subagent-tasks/wave9-topdocs-fix-20260815/topdocs_p3_fix_SPEC.md`
    （main-agent 核准）。
  - **Boundaries**: 只改 SPEC 列出的注释；禁代码改动/格式化/编译。
  - **Result (2026-08-15, ACCEPTED)**: 8/8 替换与 SPEC 逐字一致；8 个 `.rs` diff
    仅注释行；F1 顶层 doc 已按 ext2/exfat 风格补齐、P3 7 条已清；F2/F10/F11 未执行。

- **`wave9_lock_vocab_review_20260815`**
  - **Kind**: bounded Reviewer wave（user-directed 2026-08-15；1 个 Reviewer、只读）。
  - **Parent**: `N/A` — lock-domain vocabulary clarity audit（DIR/CUL/INODE/WL/UPPER/
    MOUNT/IU/BIO；全树注释）。
  - **Basis**: handoff §4.1 C1/N9/P3/P10 + book module-docs/design-decisions。
  - **Boundaries**: read-only；唯一写产物 = 1 份审计报告
    `components/wave9-lock-vocab-review-20260815/`；不执行修复。
  - **Result (2026-08-15, ACCEPTED)**: 24 findings（HIGH 3 / MED 21；define 6 /
    rewrite 16 / delete 2；no_op 6）；main agent 核实 HIGH 3 与 user 点名项；
    修复未授权，待 user 指令。

- **`wave9_lock_vocab_fix_20260815`**
  - **Kind**: bounded Creator pass（user-approved 2026-08-15；comment-only）。
  - **Parent**: `N/A` — continuation of `wave9_lock_vocab_review_20260815`.
  - **Scope**: 36 处 exact old→new，全树注释去锁缩写（DIR/CUL/INODE/WL/UPPER/
    MOUNT/BIO → 锁角色+代码字段）；新文本禁用 snapshot。
  - **Basis**: `subagent-tasks/wave9-lock-vocab-fix-20260815/lock_vocab_rewrite_SPEC.md`。
  - **Boundaries**: 只改 SPEC 列出的注释；禁代码改动/格式化/编译。
  - **Result (2026-08-15, ACCEPTED)**: 36/36 替换与 SPEC 一致；全树注释锁缩写残留
    grep=0；新增文本无 snapshot；13 个 `.rs` diff 仅注释行。

- **`wave9_principles_cleanup_20260816`**
  - **Kind**: user-directed comment-only cleanup of the wave-9 principles fullaudit
    findings（2026-08-16）。Two lanes：
    **R1 delete+simplify**：6 个 deepseek-v4-flash Creator 并行执行；
    **R2 rewrite**：6 个 deepseek-v4-pro Creator 先产提案（只写提案报告，不改 `.rs`），
    main agent 逐条核准后二次派发执行。
  - **Parent / covered micros（每 scope 一包，与 wave6 pass 22-27 同 surface，无新 feature claim）**:
    mount=`mount_resource_policy`（P0-01/02/03/05/18, P1-19/20/35, P2-11）；
    projection=`visibility_projection_identity`（P0-04/06/07/08/09/10/11/12/16/17, P1-07, P2-01）；
    dir=`namespace_mutation_whiteout`（P1-21..P1-30, P1-36）；
    copyup=`copyup_authority_file_views`（P1-01..06/08..15/32/34/37）；
    security=`metadata_security_xattr_policy`（P1-16/17/18/33）；
    top_readdir=`merged_directory_index`（P0-13/14/15, P1-31）。
  - **Finding 来源（全部 actionable 注释项）**:
    CM-01…31（projection triage；18 delete/simplify + 13 rewrite）；
    fullaudit 109（mount 29 / projection 33 / dir 8 / copyup 16 / security 16 / top_readdir 7；
    其中 delete+simplify 76、rewrite 32、adjudicate 1 挂起）；
    copyup triage retained 2（COM-1 simplify、COM-3 rewrite）。
    USER_CODES/UC 代码疑问全部不在本轮。
  - **R1 执行口径**: 每条 finding 按报告原文定位后只做 delete/simplify；行号漂移以原文为准；
    只改注释/文档行；跳过项写收据；无编译。
  - **R2 执行口径**: Pro 只出提案（目标原文 → 拟改写文本 + 原则依据），main agent 核准后才派
    `.rs` 执行；提案报告在 `components/wave9-principles-cleanup-r2-20260816/proposals/`。
  - **Explicit boundary**: 无代码行为/签名/可见性/锁改动；不改 legacy_fs.rs；无 ktest；
    编译验证仍在代码清理彻底后统一进行。
  - **R1 result (2026-08-16, ACCEPTED)**: 6/6 Flash Creator 收据在册；95/95 项
    done=0 skipped（fullaudit delete+simplify 76 + CM delete/simplify 18 + COM-1 1）；
    28 个 `.rs` diff 仅注释行（`git diff --unified=0` 非注释行 = 0；lower_id.rs
    一处代码行仅删除行尾注释）；未编译；已 amend 入最新 wave-9 WIP。
  - **R2 proposal (2026-08-16, main-agent 核准)**: 6/6 Pro 提案覆盖 46 条 rewrite
    （fullaudit 32 + CM 13 + COM-3 1），proposed=46 blocked=0；main agent 修订 5 处：
    RIDX-FULL-02 `Serve`→`Serves`；PROJECTION-FULL-04 首行回归 “Creates or reuses”；
    CM-19 去 “immutable”；CM-31 简化 guard 措辞；MOUNT-FULL-29 保留 lower 并发修改
    bullet（N2）仅去跨模块符号；MOUNT-FULL-19 bullet 保持 `-`。
  - **R2 result (2026-08-16, ACCEPTED)**: 6/6 Pro Creator 收据在册；46/46 done=0
    skipped；所有 old-phrase grep 0 残留、`/// TODO` 0 残留；`git diff --check` clean；
    diff 仅注释行；未编译；已 amend 入最新 wave-9 WIP。adjudicate 1（mount `RealPath` struct doc）仍挂起。

- **`wave9_day2_flash_cleanup_20260817`**
  - **Kind**: user-directed comment-only cleanup of the wave-9 day-2 Flash audit
    findings（2026-08-17；恢复执行）。Two lanes：
    **R1 delete+simplify**：5 个 deepseek-v4-flash Creator 并行执行
    （mount 0 条不派）；**R2 rewrite**：deepseek-v4-flash Creator 先产提案
    （只写提案报告，不改 `.rs`），main agent 逐条核准后二次派发执行。
  - **Parent / covered micros（每 scope 一包，与 `wave9_principles_cleanup_20260816`
    同 surface，无新 feature claim）**:
    mount=`mount_resource_policy`；projection=`visibility_projection_identity`；
    dir=`namespace_mutation_whiteout`；copyup=`copyup_authority_file_views`；
    security=`metadata_security_xattr_policy`；top_readdir=`merged_directory_index`。
  - **Finding 来源**: day-2 audit 54 findings 的 actionable 注释项——
    R1 delete+simplify 34（copyup 2 / dir 4 / metadata 4 / projection 22 /
    top_readdir 2；mount 0）；R2 rewrite 20（mount 3 / projection 3 / dir 8 /
    copyup 2 / metadata 4；top_readdir 0）。
  - **R1 执行口径**: 每条 finding 按 SPEC 原文定位后只做 delete/simplify；行号漂移以
    原文为准；只改注释/文档行；跳过项写收据；无编译。top_readdir 2 条为上次中断前
    已写入的 partial 改动，本轮只 verify 并记 done(pre-applied)。
  - **R2 执行口径**: Flash 只出提案（目标原文 → 拟改写 verbatim 文本 + 原则依据，
    允许 delete/simplify 降档），main agent 核准后才派 `.rs` 执行；提案在
    `components/wave9-day2-flash-cleanup-20260817/r2/proposals/`。
  - **Explicit boundary**: 无代码行为/签名/可见性/锁改动；不改 legacy_fs.rs；无 ktest；
    不 cargo fmt、不编译；编译验证仍在代码清理彻底后统一进行。
  - **Status (2026-08-17, ACCEPTED)**: R1 5/5 Creator 收据在册，34/34 闭合
    （copyup 2 / dir 4 / metadata 4 / projection 22 / top_readdir 2
    done(pre-applied)；mount 0 不派）；main agent 直接修正 copyup trigger.rs
    “is release”→“is released”一处语法。R2 提案 5/5 覆盖 20/20（proposed=20
    blocked=0）；main agent 核准并修订 2 处（DIR-D2-07 改 “the type and
    rmdir-emptiness gates”；PROJ-D2-16 改 “this caller skips the layer
    resolution”）；R2 执行 20/20 闭合（done=20 skipped=0）。全部 `.rs` diff
    非注释行 = 0；`git diff --check` clean；未编译；已 amend 入最新 wave-9 WIP。
