<!-- SPDX-License-Identifier: MPL-2.0 -->

# System Blueprint & Dispatch Ledger

This file is the scheduler-owned live board for a filesystem implementation or refactor workspace.
Managers and subagents update artifacts elsewhere; only the main agent updates this board.

## 1. Macro Topology & Global Status

- [x] **Phase 0: Agent Workflow Bootstrap**
  - **Status**: Accepted
  - **Commit**: `b2d0df22a`
  - **Accepted Artifact**: `.agents/` directory layout, protocol, skills

- [x] **Phase 1: Priors Layer**
  - **Status**: Accepted
  - **Commit**: `f03ef716b` (priors) + `45362e19f` (ra-code-nav) + `ff1c00a2f` (xfstests mapping)
  - **Accepted Artifacts**: `priors/REFERENCE_IMPLEMENTATION_SUMMARY.md`,
    `priors/FILESYSTEM_SPEC_SUMMARY.md`, `priors/FILESYSTEM_SPEC_INDEX.md`,
    `priors/MICRO_FEATURE_INVENTORY.md`, `.agents/skills/ra-code-nav/`

- [x] **Phase 2: Architect Handoff (reframed topology)** (`architect-reframed-topology`)
  - **Status**: Accepted after fresh structural audit; supersedes the deleted old topology
  - **Dispatch**: `subagent-tasks/architect-reframed-topology/architect_reframed_topology_dispatch.md`
  - **Accepted Artifacts**: `components/architect-reframed-topology/macro_00_global_topology.md` and 7 Meso architecture maps
  - **Acceptance Evidence**: 81 formal Micro IDs occur exactly once as primary owners; all maps contain the required architecture sections; the fresh `DIR -> CUL -> INODE -> WL -> UPPER` topology and Asterinas BIO/re-entry constraints are explicit; no implementation or test artifact was created
  - **Scope Classification**: 57 Micro IDs are `需要实现` (all P0/P1 plus
    `P2-01 xino` and `P2-11 UUID modes`); 24 are `暂不实现`
    (`P2-02..P2-17` minus `P2-11`, and `P3-01..P3-09`). Amended 2026-08-01:
    `P2-11` promoted to `需要实现` in `mount_resource_policy`, with the claim
    token unified with the 64-bit overlay UUID (single entity).
    Amended 2026-08-02 (user-directed topology): `P1-07` ownership moves from
    `persistent_association_export` (meso 07) to `visibility_projection_identity`
    (meso 02) — the origin/lower-id record is identity-domain, no runtime owner;
    meso 07 becomes deferred-only (0/4), and the future index feature is
    recorded as a separate small copy-up-adjacent module concept.
    Deferred IDs remain mapped future insertion points and are not
    implementation commitments in this wave.

- [x] **Phase 3: Designer Contracts** (per new Meso-component)
  - **Status**: **Complete / Closed (2026-08-03)** — 6 basic-wave Designer
    contracts `Specified` (meso 01-06); meso 07 deferred-only (0/4). Phase 4
    is open: pass slicing recorded 2026-08-03 (7 Creator passes; workflow
    amended to Reviewer-only pre-code gate + single meso-integration xfstests
    gate — no per-pass Checker).
  - **Prerequisite**: Accepted Phase 2 reframed topology
  - **Progress (2026-08-02)**: Designer wave **COMPLETE** — 6 basic-wave
    contracts are `Specified` (`mount_resource_policy` 9/6,
    `visibility_projection_identity` 12/2, `merged_directory_index` 4/1
    revision 01, `copyup_authority_file_views` 17/6 revision 01,
    `metadata_security_xattr_policy` 4/4 revision 01,
    `namespace_mutation_whiteout` 11/1 revision 01), all with validation
    deferred to meso-integration. meso 07 is **deferred-only (0/4)** (P1-07
    moved to meso 02 as the lower-id record; future index/export/nlink/
    offline-detection deferred, index recorded as a separate copy-up-adjacent
    module concept). Bounded meso-01 revisions (default_permissions
    publication; can_mknod_char + whiteout capability gate) closed the
    meso-05/06 consumption gaps. Designer contracts preserve the 57/24 scope
    labels and carry the `UPPER`/`WL` `可能清理` marker.
  - **Expected Artifacts**: designer spec/validation artifacts for meso 01-06
    (under `.agents/components/<component-id>/`; the tree is gitignored) plus
    the meso 07 deferred-only disposition; packets under `.agents/subagent-tasks/`.
  - **Boundary**: Do not reconstruct the deleted 13-Meso specs/validation contracts or create replacement Designer artifacts during this topology reset. The 57/24 classification is a design-scope decision only; it does not start Designer work.

- [x] **Phase 4: Creator Pass Slicing + Implementation** (per Creator Pass)
  - **Status**: **Complete (2026-08-03)** — all four implementation Waves
    executed and ACCEPTED under the per-file Creator orchestration: Wave 1
    `mount/` (9 micros), Wave 2 `projection/` + mount extensions (12 micros),
    Wave 3 shared-carrier seams (no feature claims), Wave 4 leaf mesos
    `readdir_index.rs`/`copyup/`/`metadata_security/`/`dir/` (36 micros).
    Each wave ran the aster-code-review diff-mode gate (3-persona fan-out)
    with long-lived repair loops; the 57-micro `需要实现` set is fully
    implemented across 30 new `.rs` files (~10k lines). Stable commits:
    `e1613f12c` (W1), `77b0d4a49` (W2), `b9b9d6caf` (W3), `43a0747bc` (W4).
  - **Workflow amendment (user-directed 2026-08-03)**: no Creator-synced
    per-pass Checker passes; Reviewer is the only pre-code-completion static
    gate; the single runtime gate is the meso-integration xfstests Checker
    after implementation + Reviewer stabilize. Creator passes are
    command-free with no per-pass compile preflight.
  - **Prerequisite**: Accepted Phase 3 Designer contracts (met) +
    `creator_pass_slicing_20260803` decision (met).
  - **Expected Artifacts**: Creator receipts
    `components/<component-id>/pass_XX_<component>_creator.md` + production
    `.rs` files under `kernel/src/fs/fs_impls/overlayfs/`; later per-meso
    Reviewer reports; final meso-integration Checker evidence per the six
    Designer validation contracts.
  - **Legacy file (2026-08-03, user-directed):** `fs.rs` renamed to
    `legacy_fs.rs` (`mod.rs` updated; content frozen); the legacy
    `OverlayFsType` registration stays active until takeover. The only
    permitted reference to `legacy_fs.rs` in Creator/Reviewer packets is the
    registration wiring; all other legacy content is off-limits as a source.
  - **Pre-Wave5 closure gate (2026-08-04, user-directed): ACCEPTED.** The
    structurally accepted addendum under `components/pre_wave5_closure/` was
    implemented serially by one Creator and exact-diff accepted by the main
    agent: the full six-file C2 typed `EEXIST` retry, P3's native origin
    triplet and unique pair resolution, and all six mechanical repairs.
    Accepted Rust changes were amended into `7aabd029c`; P1/P2 remain deferred
    known gaps. Wave5's Checker-owned static compile/lint lane is now open;
    no Reviewer was introduced for this bounded closure.
  - **Wave5 static entry final result (2026-08-04): BLOCKED / USER DECISION.**
    The three exact target-specific Checker runs reduced the compile result
    from 42 errors / 16 warnings to 15 errors / 1 warning; every authorized
    mechanical repair is amended at `7aabd029c`. The remaining five categories
    require explicit authority: a viable root-inode one-shot carrier,
    trait-implementation ownership/delegation, `FsCreationCtx` task-context
    access, in-use claim-slot lifecycle, and the source-permission
    `AccessType`. `make kernel`, `make check`, runtime, and xfstests were not
    run; no overlayfs-local substitute or further repair is authorized.
  - **Wave5 five-item code-form reconciliation (2026-08-04): DESIGNER
    ACCEPTED.** `task_designer_wave5_static_owner_reconciliation_20260804`
    produced the accepted specification and validation contract under
    `components/wave_05_static_repair_design/`. They freeze the user-fixed
    root slot, sole trait owner/forwarders, VFS task context, dedicated inode
    extension in-use slot, and `ReadOnly` source admission. The VFS production
    write-set remains exactly `fs_apis/{registry.rs,inode.rs,inode_ext.rs}`;
    it reuses the existing `Mutex`, Extension/InodeExt pattern, and atomics.
    No implementation or validation command is authorized by this acceptance.
  - **Wave5 takeover (2026-08-04, user-directed):** the active registration
    names `mount::OverlayFsType`; the legacy module is no longer linked,
    though `legacy_fs.rs` remains an archive. The final static evidence is
    `components/wave_05_compile_lint/pass_11_wave5_compile_lint_checker.md`.

## 2. Meso-Component Pipeline Index

| Meso-Component | Current Scope | 1. Architect Map | 2. Designer Contract | 3. Creator Passes | 4. Checker Passes | 5. Integration Pass | 6. Reviewer | Overall Status |
| :------------- | :----------: | :-------------: | :------------------: | :---------------: | :---------------: | :-----------------: | :---------: | :------------- |
| `mount_resource_policy` | 9 / 6 | Accepted | **Accepted** (2026-08-01) | **Done** `pass_01` (Wave 1, accepted) | Not scheduled (workflow amendment) | Pending (integration) | Pending (integration) | **Implemented** |
| `visibility_projection_identity` | 12 / 2 | Accepted | **Accepted** (2026-08-01) | **Done** `pass_02` (Wave 2, accepted) | Not scheduled (workflow amendment) | Pending (integration) | Pending (integration) | **Implemented** |
| `merged_directory_index` | 4 / 1 | Accepted | **Accepted** (2026-08-02, revision 01) | **Done** `pass_04` (Wave 4, accepted) | Not scheduled (workflow amendment) | Pending (integration) | Pending (integration) | **Implemented** |
| `copyup_authority_file_views` | 17 / 6 | Accepted | **Accepted** (2026-08-02, revision 01) | **Done** `pass_05` (Wave 4, accepted) | Not scheduled (workflow amendment) | Pending (integration) | Pending (integration) | **Implemented** |
| `metadata_security_xattr_policy` | 4 / 4 | Accepted | **Accepted** (2026-08-02, revision 01) | **Done** `pass_06` (Wave 4, accepted) | Not scheduled (workflow amendment) | Pending (integration) | Pending (integration) | **Implemented** |
| `namespace_mutation_whiteout` | 11 / 1 | Accepted | **Accepted** (2026-08-02, revision 01) | **Done** `pass_07` (Wave 4, accepted) | Not scheduled (workflow amendment) | Pending (integration) | Pending (integration) | **Implemented** |
| `persistent_association_export` | 0 / 4 | Accepted | **Deferred-only** (2026-08-02; P1-07 moved to meso 02; no basic-wave Designer contract) | - | - | - | - | Deferred |

`pass_03_shared_carrier_seams` (Wave 3) is a cross-meso foundation pass (parent
N/A, no feature claims); it is recorded in `PASS_SLICING.md` but is not a
meso-component row. Per the 2026-08-03 workflow amendment, the per-pass Checker
column is intentionally empty for this wave; runtime validation is the single
meso-integration xfstests Checker (Wave 6) per the six Designer validation
contracts.

`Current Scope` is shown as `需要实现 / 暂不实现`; the seven rows sum to
`57 / 24` across the complete 81-Micro inventory (amended 2026-08-01).

_The old 13-Meso topology and its Designer dispatch/specification artifacts are
discarded. Phase 4 pass slicing is recorded (2026-08-03); no implementation
pass is dispatched yet. The next action is Creator dispatch per the live
handoff `main-agent/20260803-creator-pass-slicing_main_agent_handoff.md`. The
next design action, if authorized, is a fresh Designer wave under
the seven accepted Meso boundaries._

## 3. Active Pass Tracking

Record only active or recently changed passes here. Durable slicing rationale belongs in `PASS_SLICING.md`.

- **`pass_00_old_ovfs_baseline_test`** — **Complete / Baseline Evidence Only**
  - **Kind**: Pre-design legacy baseline Checker pass; not an implementation pass
  - **Parent**: `legacy_overlayfs_baseline_validation` (temporary validation lane, not an Architect meso-component)
  - **Scope**: `overlay/001`–`overlay/078`, `overlay/100`, and `overlay/101`; classify every case as `PASS`, `FAIL`, `CORRUPT`, `NOTRUN`, or `HANG/TIMEOUT`
  - **Dispatch**: Follow-up agent `019f7f02-e0a0-7c90-9b4f-9083691caa6b` completed the attributable rerun; prior diagnostic agent `019f7ebb-09f6-7c22-b31a-1157328a91ea` is also closed
  - **Execution shape**: One fresh QEMU and freshly recreated TEST/SCRATCH image pair per case, a 300-second hang cutoff, and immediate append to the rerun status table
  - **Result**: 80 attributable rows: 9 `PASS`, 15 `FAIL`, 49 `NOTRUN`, and 7 `HANG/TIMEOUT`; no `CORRUPT`. The Architect design wave is now unblocked as a separate user-authorized scheduling decision.

## 4. Open Escalations / Notes

- Protocol decision recorded for the design wave: Macro/Meso/Micro remain the
  owner, lock-topology, semantic-boundary, traceability, and scheduling
  hierarchy; they are not xfstests granularity levels. xfstests is treated as
  a many-to-many external evidence mapping.
- The Designer validation contract is simplified to that external mapping plus
  runtime invariant and meso-integration observations. Creator-synced Checker
  passes retain exact code/micro scope for failure attribution, but do not
  require one test per micro-feature or any ktest-based validation. This
  refactor uses xfstests only.

- The legacy baseline is intentionally scheduled before the Architect wave, following the closed priors handoff. It does not advance Phase 2 or Phase 3 and cannot accept refactor implementation work.
- User-directed execution constraint satisfied: one QEMU and freshly recreated TEST/SCRATCH images were used per case; no shared batch was used.
- Diagnostic checkpoint: the initial 34 rows are provisional and invalid because image freshness was not proven and post-QEMU mount disappearance was misclassified as `CORRUPT`.
- Controlled rerun complete: classification used only each case's `qemu.log`; no `CORRUPT` was recorded. The one-hour controller ceiling was reconciled from the uniquely attributable `overlay/076` log, and the final 80-row matrix is preserved in `.agents/subagent-tasks/old-ovfs-baseline-test/evidence/baseline_case_matrix_rerun.tsv`.
- The previously accepted 13-Meso Architect/Designer wave is superseded by a
  user-directed topology reset. Its old component directories and old
  Designer dispatch packets were deleted; the retained old-OVFS baseline is
  independent evidence and is not an accepted refactor component.
- The fresh Architect output contains one Macro and seven Meso maps under
  `components/architect-reframed-topology/`. A mechanical audit found all 81
  formal Micro IDs exactly once, no duplicate or missing primary owner, no
  `P3-10`, and complete Macro/Meso architecture sections.
- No Designer spec/validation, Creator/Checker/Reviewer packet, production
  code, test, ktest, xfstests harness, build, or runtime artifact was created
  in the topology reset. Phase 3 remains intentionally unstarted.
- The global lock hierarchy accepted for this wave is
  `DIR -> CUL -> INODE -> WL -> UPPER`; `IU` remains an out-of-band mount-time
  upper/workdir claim protocol. BIO-capable paths use sleep-capable Mutex
  domains, and VFS dentry children guards are not parent locks.
- B/C-1 records two unresolved upper/workdir claim-carrier options: an
  inode-owned `Extension` runtime lease and a persistent xattr reservation.
  Neither is treated as an existing Asterinas capability or frozen API.
- Stage-D scope is now classified explicitly: implement all P0/P1 Micro IDs and
  `P2-01 xino` and `P2-11 UUID modes` (57 total); defer `P2-02..P2-17` minus `P2-11`, and `P3-01..P3-09` (24 total).
  The deferred set remains visible in the seven architecture maps for future
  Designer decisions, but no deferred Micro is part of the current
  implementation promise.
- This classification does not create Creator/Checker/Reviewer passes. Phase 3
  remains `Not started`, and any future Designer wave must use the seven Meso
  boundaries and preserve the same scope labels.
- **Lock-topology discussion closed (2026-08-01):** The five operation-path
  locks were verified against `/home/ayd/linux` (7.2.0-rc3): `DIR` is the
  Asterinas substitute for the role of Linux's VFS-held parent `i_rwsem`;
  `INODE`/`WL`/`IU` map to `ovl_inode->lock`/`whiteout_lock`/`I_OVL_INUSE`;
  `CUL` and `UPPER` have no direct Linux counterpart. `UPPER` (and possibly
  `WL`) are marked `可能清理` (cleanup candidates) and deferred to the
  Designer/Creator stages; no convergence list or artifact repair is active.
  Phase 3 Designer dispatch is the next main-agent action; see the live
  handoff `main-agent/20260801-state-cleanup-designer-prep_main_agent_handoff.md`.
- **meso 01 runtime validation deferred (2026-08-01):** the mount group's
  xfstests evidence moves to a meso-integration obligation after the identity
  Meso's root carrier + minimal read path exist; the meso 01 Creator pass is
  accepted on structural + compile-preflight evidence only.
- **Workflow amendment (user-directed 2026-08-03):** Creator-synced per-pass
  Checker passes are eliminated for this wave; the Reviewer is the only
  pre-code-completion quality gate (static; PROTOCOL §1 rule 16 pre-checker
  structural audit, user-requested); the meso-integration xfstests Checker is
  the single runtime gate after implementation + Reviewer stabilize. Creator
  passes are command-free with no per-pass compile preflight. Recorded in
  `PASS_SLICING.md` (`creator_pass_slicing_20260803`) and the live handoff
  `main-agent/20260803-creator-pass-slicing_main_agent_handoff.md`; PROTOCOL
  §1 rule 5 remains unedited pending user confirmation of a permanent
  amendment.
