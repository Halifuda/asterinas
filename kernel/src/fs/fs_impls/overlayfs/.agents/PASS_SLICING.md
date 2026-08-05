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
    `kernel/src/fs/vfs/fs_apis/{registry.rs,inode.rs,inode_ext.rs}`. It adds no
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
    `kernel/src/fs/fs_impls/overlayfs/` understands the code without the
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
