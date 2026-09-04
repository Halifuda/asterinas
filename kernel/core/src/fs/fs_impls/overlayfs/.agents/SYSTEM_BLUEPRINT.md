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
  - **Wave7 + Wave8 closure (2026-08-12):** Wave7 pass_40–pass_45 全部 gate 已接受；Wave8 静态门（rustfmt + clippy `-Dwarnings` 全绿）、全量 review（74 条 → C1–C11 全部落地）、以及 **20 例可调度矩阵全量复测 PASS（2026-08-11，user-confirmed；029 修复后无次生回归）** 全部完成。**Wave8 CLOSED**（handoff: `main-agent/20260811-wave8-format-lint_main_agent_handoff.md` §4.14）。
  - **PR delivery phase (2026-08-12, open):** 从 pure upstream main（`94a8f624d`）切干净分支，交付「新 overlayfs + 配套 VFS 修改 + legacy 删除」，按 11-commit（1k~2k 行/commit）切分（story-first：C2–C10 惰性文件、C11 合龙）。活跃 handoff: `main-agent/20260812-pr-draft-prep_main_agent_handoff.md`。此为交付形态切分，不改变本 board 的 meso/micro pass 账目。
  - **Workflow amendment (user-directed 2026-08-03)**: no Creator-synced
    per-pass Checker passes; Reviewer is the only pre-code-completion static
    gate; the single runtime gate is the meso-integration xfstests Checker
    after implementation + Reviewer stabilize. Creator passes are
    command-free with no per-pass compile preflight.
  - **Prerequisite**: Accepted Phase 3 Designer contracts (met) +
    `creator_pass_slicing_20260803` decision (met).
  - **Expected Artifacts**: Creator receipts
    `components/<component-id>/pass_XX_<component>_creator.md` + production
    `.rs` files under `kernel/core/src/fs/fs_impls/overlayfs/`; later per-meso
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
  - **Wave5 static entry initial stop record (2026-08-04): superseded by the
    accepted user decisions and implementation below.**
    The three exact target-specific Checker runs reduced the compile result
    from 42 errors / 16 warnings to 15 errors / 1 warning; every authorized
    mechanical repair is amended at `7aabd029c`. The remaining five categories
    require explicit authority: a viable root-inode one-shot carrier,
    trait-implementation ownership/delegation, `FsCreationCtx` task-context
    access, in-use claim-slot lifecycle, and the source-permission
    `AccessType`. `make kernel`, `make check`, runtime, and xfstests were not
    run at that point; no overlayfs-local substitute was authorized.
  - **Wave5 five-item code-form reconciliation (2026-08-04): DESIGNER
    ACCEPTED.** `task_designer_wave5_static_owner_reconciliation_20260804`
    produced the accepted specification and validation contract under
    `components/wave_05_static_repair_design/`. They freeze the user-fixed
    root slot, sole trait owner/forwarders, VFS task context, dedicated inode
    extension in-use slot, and `ReadOnly` source admission. The VFS production
    write-set remains exactly `fs_apis/{registry.rs,inode.rs,inode_ext.rs}`;
    it reuses the existing `Mutex`, Extension/InodeExt pattern, and atomics.
    No implementation or validation command is authorized by this acceptance.
  - **Wave5 static entry continuation 04 (2026-08-04): REPRODUCED / STILL
    BLOCKED.** After amend `783c81041`, the Checker ran the same sole
    target-specific container `cargo check`; it exited `101` with the
    unchanged 15 errors and one warning. The receipt proves the accepted
    reconciliation is documentation only: its root slot, canonical trait
    forwarders, three-file VFS seam, and `ReadOnly` source call have not yet
    landed in production. No new mechanical repair is authorized; `make`,
    Clippy, runtime, and xfstests remain unscheduled.
  - **Wave5 five-item implementation (2026-08-04): ACCEPTED.** The five
    separately reviewed Creator repairs
    landed as the mutex-option root publication, canonical trait owner with
    sibling `*_impl` helpers, narrow `FsCreationCtx::task_ctx`, dedicated VFS
    in-use slot, and read-only link-source admission. The Rust-only result was
    amended to `1378d502a`. Continuation 05 then ran the exact container
    target-specific `cargo check` once at its preceding amend `10cf627e2` and
    reduced the result to five errors and one warning. The main agent amended
    its three explicitly mechanical candidates (two imports and an unused
    local rename) without rerunning. The subsequent user-approved ownership
    repair evaluated opacity / `d_type` before moving the affected values,
    preserving the existing barrier and readdir contracts; continuation 06
    confirmed the three `E0382` errors were gone. Its two remaining errors were
    a direct claim-type visibility propagation, amended as the single
    `UpperWorkdirClaim` ceiling widening at `36c30ac33`. Continuation 07 then
    passed the exact target-specific container `cargo check` (exit 0, 8.54s).
    It emits 17 warnings, so this is not `make check` or lint acceptance;
    `make kernel`, `make check`, runtime, and xfstests remain unscheduled.
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

- **`wave6_documentation_review_20260805`** — **Active / Static closure ACCEPTED**
  - **Kind**: User-directed comprehensive comment-documentation review wave
    (pass-slicing recorded in `PASS_SLICING.md`); supersedes the lint-only
    Wave6 framing. The nine Clippy diagnostics and the two TODO annotations
    are mandatory items inside passes 22/24.
  - **Scope**: all 31 active `.rs` files (everything except `legacy_fs.rs`);
    remove all micro-feature IDs and internal workspace vocabulary from
    comments, rewrite stale/redundant prose against current code, fix the
    nine rustdoc diagnostics, add the two scoped comment-only TODOs. Shared
    standard: `subagent-tasks/wave_06_documentation_review/
    WAVE6_DOC_STANDARD.md`.
  - **Passes**: `pass_22_mount_resource_policy` (mount/* + crate-root annex),
    `pass_23_visibility_projection_identity` (projection/*),
    `pass_24_namespace_mutation_whiteout` (dir/*),
    `pass_25_copyup_authority_file_views` (copyup/*),
    `pass_26_metadata_security_xattr_policy` (metadata_security/*),
    `pass_27_merged_directory_index` (readdir_index.rs). All command-free,
    risk Low; main-agent exact-diff acceptance per lane.
  - **Post-lane gates**: Checker workspace Clippy → rustfmt → `make check`
    (each run evidenced). **Closed 2026-08-05:** workspace Clippy (exit 0),
    `cargo fmt --check` (exit 0), `make check` (exit 0) after the authorized
    mechanical `cargo fmt` (14 files incl. VFS `inode_ext.rs`) and
    trailing-whitespace cleanup of 11 pre-existing `.agents/` markdown files;
    evidence under `components/wave_06_documentation_review/run_evidence/`.
    A second review pass (2026-08-05) cleaned doc comments that dwelt on
    visibility/publish/derive/mechanical minutiae instead of meaning and
    design intent; the static chain was revalidated (runs 06-08, all exit 0).
    Runtime xfstests, final Reviewer acceptance, and Wave7 `legacy_fs.rs`
    deletion remain later gates.

- **`wave9_comments_abflex_audit_20260815`** — **Active / dispatched 2026-08-15**
  - **Kind**: bounded Reviewer wave（comments A/B-flexibility full-tree
    audit）；6 个并行只读 Reviewer（mount / projection / dir / copyup /
    security / top_readdir）；无 `.rs` 写权限；不执行清理。
  - **Basis**: user 指示按 A/B 两档灵活性原则
    （N1/N2/C2/C3/P2/N10/N11 + N9/N5/C4/C1/P10/P5/P9′）对全树现存 comments
    再全面 Review；PR3708 A/B 档 finding（N9/N5/P5/P2）复核后按
    `REMAINING` 再报；C/D 档不正式报。
  - **Artifacts**: 6 份审计报告 →
    `components/wave9-comments-abflex-audit-20260815/`；criteria/packets →
    `subagent-tasks/wave9-comments-abflex-audit-20260815/`。
  - **Result**: **ACCEPTED 2026-08-15** — 6/6 报告验收通过；13 findings
    （HIGH 1/MED 2/LOW 10；N9×6/N5×4/P5×1/P2×1/C1×1；REMAINING 10 /
    NEW 3 / REGRESSION 0）；无 `.rs` 改动；cleanup 待 user 指令。

- **`wave9_comments_abflex_exec_20260815`** — **ACCEPTED 2026-08-15**
  - **Kind**: two-lane execution wave（user-directed）。
  - **Lane 1**: deepseek-v4-flash Creator 机械档 3/3 精确执行（M-AB-3 /
    CP-AB-1 / D-AB-1）；diff 与 packet 一致，0 代码行改动。
  - **Lane 2**: 两个 deepseek-v4-pro 对灵活档 10 条 A 提案 → B 批驳
    （6 修正/4 同意）→ A 终稿；`flexible_plan_final.md` 10/10 覆盖并附
    main-agent 验收注记。**user 已批准终稿，4 个 deepseek-v4-flash 按
    `flexible_plan_exec_SPEC.md` 执行 10/10 并验收**；9 个 `.rs` diff
    仅注释行、0 代码改动；未编译未提交。
  - **Artifacts**: `components/wave9-comments-abflex-exec-20260815/`；
    packets `subagent-tasks/wave9-comments-abflex-exec-20260815/`。

- **`wave9_topdocs_fix_20260815`** — **ACCEPTED 2026-08-15**
  - **Kind**: bounded Creator pass（comment-only）。
  - **Result**: F1 顶层 `overlayfs/mod.rs` `//!` doc 已按 ext2/exfat 风格补齐；
    P3 7 条（F3–F9）已执行；8/8 与 SPEC 逐字一致、diff 仅注释行。
    F2/F10/F11 待 user 指令；未编译未提交。

- **`wave9_lock_vocab_fix_20260815`** — **ACCEPTED 2026-08-15**
  - **Kind**: bounded Creator pass（comment-only）。
  - **Result**: 36/36 exact old/new 完成；全树注释锁缩写残留 grep=0；新文本无
    snapshot；13 个 `.rs` diff 仅注释行；未编译未提交。

- **`wave9_lock_vocab_review_20260815`** — **ACCEPTED 2026-08-15**
  - **Kind**: bounded Reviewer wave（1 个 Reviewer、只读）。
  - **Result**: 24 findings（HIGH 3：UPPER/MOUNT 无对应代码锁、copyup Lock contract
    缺 CUL；MED 21 含 dir/whiteout WL/DIR、WL 全局定义缺失、BIO）；main agent 核实；
    修复待 user 指令。
  - **Artifacts**: `components/wave9-lock-vocab-review-20260815/`；
    packets `subagent-tasks/wave9-lock-vocab-review-20260815/`。

- **`wave9_topdocs_review_20260815`** — **ACCEPTED 2026-08-15**
  - **Kind**: bounded Reviewer wave（1 个 Reviewer、只读）。
  - **Basis**: book guides（maintainability comments、rust-specific
    comments/crates-and-modules、documentation general-style）。
  - **Result**: 11 findings（module-docs 2 NEW；P3 7 REMAINING；
    no-impl-in-docs 1；explain-why 冗余 1）；main agent 逐条锚定核实；
    修复未授权，待 user 指令。
  - **Artifacts**: `components/wave9-topdocs-review-20260815/`；
    packets `subagent-tasks/wave9-topdocs-review-20260815/`。

- **`wave9_projection_user_annotations_triage_20260816`** — **ACCEPTED 2026-08-16**
  - **Kind**: bounded Reviewer triage（1 个 deepseek-v4-flash、只读；workflow 直派，
    provider `opencode-go` + model `deepseek-v4-flash`）。
  - **Scope**: projection 7 文件工作树 user 标注 53 条（31 `USER_COMMENTS` +
    22 `USER_CODES`）。
  - **Result**: 两份报告验收通过——
    `components/wave9-projection-user-annotations-20260816/
    user_codes_summary_20260816.md`（UC-01…22，全部「留待后续处理」）与
    `user_comments_principle_analysis_20260816.md`（CM-01…31，逐条原则判定 +
    以前未发现根因）。projection 7 文件已 `git restore` 到 HEAD；
    **copyup/mod.rs 的 user 进行中标注未动**。清理执行与 UC 处置待 user 指令。
  - **Artifacts**: `components/wave9-projection-user-annotations-20260816/`；
    packet `subagent-tasks/wave9-projection-user-annotations-20260816/`。

- **`wave9_principles_fullaudit_20260816`** — **Dispatched 2026-08-16（6 个 Reviewer 并行，只读）**
  - **Kind**: bounded Reviewer wave；按 §4.26 projection triage 的 31 条 CM 判定提炼为本轮判据
    （P2/C2/C3 复述自解释、N4/N1/C4 叙述质量、N5 文档边界、N9/P10/P3/N8 模块 doc/术语、
    N2/P4 rationale、C1 锁注释、C6 阈值），对 overlayfs 全树 31 个 active `.rs` 的 **git HEAD**
    版本再审查。
  - **Scopes**: mount（`mod.rs` + `mount/*`）、projection、dir、copyup、security、top_readdir；
    projection 的 CM-01…31 为已知项不重复报。不审计工作树中 user 进行中的标注。
  - **Criteria/packets**: `subagent-tasks/wave9-principles-fullaudit-20260816/`；
    报告目标 `components/wave9-principles-fullaudit-20260816/`。无 `.rs` 写权限、不执行清理。
  - **Result**: **ACCEPTED 2026-08-16（结构验收）** — 6/6 报告收齐、计数闭合、无 `.rs` 改动。
    **109 findings**（NEW 96 / REMAINING 13 / REGRESSION 0；HIGH 1 / MED 15 / LOW 93）：
    mount 29（22/7/0；0/8/21）、projection 33（32/1/0；0/3/30，CM-01…31 已排除）、
    dir 8（7/1/0；1/0/7）、copyup 16（16/0/0；0/1/15）、
    security 16（12/4/0；0/3/13）、top_readdir 7（7/0/0；0/0/7）。
    处置 delete 23 / simplify 53 / rewrite 32 / adjudicate 1；执行待 user 指令。

- **`wave9_copyup_user_annotations_triage_20260816`** — **ACCEPTED 2026-08-16**
  - **Kind**: bounded Reviewer triage（1 个 deepseek-v4-flash、只读；workflow 直派，
    provider `opencode-go` + model `deepseek-v4-flash`）。
  - **Scope**: copyup 工作树 user 标注 10 条（3 `USER_COMMENTS` + 7 `USER_CODES`）。
  - **Result**: 两份报告验收通过——
    `components/wave9-copyup-user-annotations-20260816/
    user_codes_summary_copyup_20260816.md`（UC-01…07，全部「留待后续处理」）与
    `user_comments_vs_fullaudit_copyup_20260816.md`（COMMENTS 3 条 → retained 2 /
    covered 1：COM-1/COM-3 新增待办，COM-2 = COPYUP-FULL-01 不重复保留）。
    工作树标注未删除（复原待 user 指令）；处置执行待 user 指令。
  - **Artifacts**: `components/wave9-copyup-user-annotations-20260816/`；
    packet `subagent-tasks/wave9-copyup-user-annotations-20260816/`。

- **`wave9_principles_cleanup_20260816`** — **ACCEPTED 2026-08-16（R1+R2 两轮执行）**
  - **Kind**: user-directed comment-only cleanup（two lanes：R1 delete+simplify，
    R2 rewrite 先提案后执行）。Slicing 见 `PASS_SLICING.md`
    `wave9_principles_cleanup_20260816`。
  - **R1**: 6 个 deepseek-v4-flash Creator 并行，执行 95 项 delete+simplify
    （fullaudit 76 + CM 18 + COM-1 1）——6/6 收据、95/95 done、0 skipped。
  - **R2**: 6 个 deepseek-v4-pro Creator 先产 46 条 rewrite 提案（fullaudit 32 +
    CM 13 + COM-3 1），main agent 核准（5 处修订），再并行执行——6/6 收据、
    46/46 done、0 skipped。
  - **验收**: 28 个 `.rs` diff 仅注释行（lower_id.rs 仅删代码行尾注释）；
    所有 old-phrase grep 0 残留；`/// TODO` 0 残留；`git diff --check` clean；
    未编译；已 amend 入最新 wave-9 WIP。adjudicate 1（mount `RealPath` struct doc）仍挂起；
    USER_CODES/UC 代码疑问仍留待后续。
  - **Artifacts**: packets/Spec/approval
    `subagent-tasks/wave9-principles-cleanup-r1-20260816/`、
    `...-r2-20260816/`；提案/收据
    `components/wave9-principles-cleanup-r1-20260816/`、
    `...-r2-20260816/`。

- **`wave9_metadata_user_annotations_triage_20260816`** — **ACCEPTED 2026-08-17**
  - **Kind**: bounded Reviewer triage（1 个 deepseek-v4-flash、只读；workflow 直派）。
  - **Scope**: metadata_security 4 文件工作树 user 标注 16 条
    （12 `USER_COMMENTS` + 4 `USER_CODES`）。
  - **Result**: 两份报告验收通过——
    `components/wave9-metadata-user-annotations-20260816/
    user_comments_summary_metadata_security_20260816.md`（12/12 归纳，初步原则
    关联但注明不裁决）与 `user_codes_summary_metadata_security_20260816.md`
    （4/4 代码疑问，留待后续）。工作树标注未删除；对比/裁决/清理待 user 指令。
  - **Artifacts**: `components/wave9-metadata-user-annotations-20260816/`；
    packet `subagent-tasks/wave9-metadata-user-annotations-20260816/`。

- **`wave9_allmodules_principles_audit_20260817`** — **ACCEPTED 2026-08-17**
  - **Kind**: bounded Reviewer wave（6 个 deepseek-v4-pro、只读；metadata_security 先单独
    审，mount/projection/dir/copyup/top_readdir 并行补派）。
  - **Basis**: user 在 metadata_security 新发现的 12 条 comment 问题归纳为 M1–M9
    并映射到 N9/P10/N2/N4/N10/P2/C2/C5/N12/C6 等原则；共享判据
    `subagent-tasks/wave9-allmodules-principles-audit-20260817/`。
  - **Result**: 六 scope 合计 **70 findings**（NEW 58 / REMAINING 12 / REGRESSION 0；
    HIGH 1 / MED 14 / LOW 55）：metadata_security 18（6/12/0；1/9/8）、mount 12、
    projection 12、dir 12、copyup 5、top_readdir 11。0 `.rs` 改动；
    清理执行待 user 指令。
  - **Artifacts**: `components/wave9-metadata-principles-audit-20260817/` +
    `components/wave9-allmodules-principles-audit-20260817/`；packets 同对应
    `subagent-tasks/` 目录。

- **`wave9_flash_cleanup_20260817`** — **ACCEPTED 2026-08-17**
  - **Kind**: user-directed two-phase comment-only cleanup，**全部 deepseek-v4-flash**
    （不用 pro）：R1 delete+simplify 36 项；R2 rewrite/add 提案 34 条 → main agent
    核准（4 处修订）→ Flash 执行 34 项。
  - **Result**: 70/70 actionable findings 闭合（36+34；0 skipped）；`git diff
    --unified=0` 非注释 `.rs` 行 = 0；关键 old-phrase grep 0 残留；方法名
    `apply_capability_gates` 未改名；未编译、未 amend（待 user 指令）。
  - **Artifacts**: packets/Spec/approval
    `subagent-tasks/wave9-flash-cleanup-20260817/{r1,r2}/`；提案/收据
    `components/wave9-flash-cleanup-20260817/{r1,r2}/`。

- **`wave9_day2_flash_audit_20260817`** — **ACCEPTED 2026-08-17**
  - **Kind**: bounded Reviewer wave（6 个 deepseek-v4-flash、只读、**只审注释/文档**）。
  - **Basis**: 昨日（2026-08-16）P2/C2/C3/N4/N5/N9/P10/P3/N8/N12/C1/C6 模式 +
    今日 M1–M9 模式；审计 git HEAD `bfeedeb69`；已闭合项不重复报。
  - **Result**: 六 scope 合计 **54 findings**（NEW 54 / REMAINING 0 / REGRESSION 0；
    HIGH 0 / MED 11 / LOW 43）：metadata 8、mount 3、projection 25、dir 12、
    copyup 4、top_readdir 2。0 `.rs` 改动。
  - **Artifacts**: `components/wave9-day2-flash-audit-20260817/`；
    packets/criteria `subagent-tasks/wave9-day2-flash-audit-20260817/`。

- **`wave9_day2_flash_cleanup_20260817`** — **ACCEPTED 2026-08-17（恢复执行并闭合）**
  - **Kind**: user-directed comment-only cleanup of day-2 audit findings；
    **全部 deepseek-v4-flash**（`provider: opencode-go` +
    `model: deepseek-v4-flash`，workflow 直派）。R1 delete+simplify 34 项
    （5 个 Creator 并行；mount 0 条；top_readdir 2 条为中断前 partial 改动，
    本轮 verify 后记 done(pre-applied)）；R2 rewrite 20 条 → Flash 提案
    （5/5 proposed、0 blocked）→ main agent 核准（dir/projection 各 1 处修订）
    → Flash 执行。
  - **Result**: **54/54 findings 闭合**（R1 34 + R2 20；done 54 / skipped 0）；
    `git diff --unified=0` 全部 `.rs` 非注释行 = 0；`git diff --check` clean；
    关键 old-phrase grep 0 残留（剩余 permission.rs 模块 doc 的 “admission”
    与代码日志中的 “impure-marker” 均不在本 wave 清单）；C6 目标块均压至
    ≤8 行；未编译；已 amend 入最新 wave-9 WIP。
  - **Artifacts**: packets/Spec/approval
    `subagent-tasks/wave9-day2-flash-cleanup-20260817/{r1,r2}/`；提案/收据
    `components/wave9-day2-flash-cleanup-20260817/{r1,r2}/`。

- **`task_creator_overlayfs_g8_mechanical_20260818`** — **ACCEPTED 2026-08-18（结构验收 + run06 PASS）**
  - **Kind**: bounded Creator wave（G8 机械子集 12 OBJ；user-directed 2026-08-18；
    provider `opencode-go` + model `deepseek-v4-flash`，workflow 直派）。
  - **Scope**: S-52/S-63/S-36/S-42/S-49/S-41/S-53/S-39/S-54/S-45/S-60/S-67
    （v2 G-6/G-4/G-1/G-8/G-10/D-8/G-7/G-3/G-11/G-9/H-1/H-12）。
    G-2/G-5/H-2/D-12/G-12/H-5/H-8 七个设计/裁决项不派；七项方向裁决见
    handoff §3.1（user-confirmed：S-29 不做；S-62 只 2/3、第三点方案 A；
    S-59 保留 CSPRNG 随机命名、删 composite+serial）。
  - **Boundaries**: 只改 packet write-set 20 个 `.rs` + 1 份收据；compile
    withheld；不改 legacy_fs.rs；无 ktest。
  - **Result**: 12/12 闭合（OBJ-12(f) done(pre-applied)）；main agent 验收
    diff clean、grep 通过，并修复 1 处顺带未用 import；已随 user 指令
    amend 进 WIP。
  - **Next**: run06 **PASS**（2026-08-18，main-agent 直跑 `cargo check`，
    exit 0）；方向项 13 个 `.rs` 已随 user 指令 amend。
  - **Artifacts**: packet `subagent-tasks/code-structure-cleanup/
    task_creator_overlayfs_g8_mechanical_20260818_dispatch.md`；收据
    `components/code-structure-cleanup/g8_mechanical_20260818/`。

- **`task_creator_overlayfs_g8_direction_step1_20260818`** — **ACCEPTED 2026-08-18（结构验收 + run06 PASS）**
  - **Kind**: bounded Creator wave（G8 方向 Step 1；opencode-go +
    deepseek-v4-flash）。
  - **Scope**: S-64/S-65/S-66/S-62(2,3 方案 A)。S-62 第 1 点与 S-29 不派。
  - **Result**: 4/4 done；main agent 验收 diff clean、grep 通过；compile
    withheld → run06 PASS（exit 0）；已随 user 指令 amend。
  - **Artifacts**: `subagent-tasks/code-structure-cleanup/
    task_creator_overlayfs_g8_direction_step1_20260818_dispatch.md`；收据
    `components/code-structure-cleanup/g8_direction_step1_20260818/`。

- **`task_creator_overlayfs_g8_direction_step2_20260818`** — **ACCEPTED 2026-08-18（结构验收 + run06 PASS）**
  - **Kind**: bounded Creator wave（G8 方向 Step 2；opencode-go +
    deepseek-v4-flash，未触发周限额）。
  - **Scope**: S-59（CSPRNG 命名统一，删 composite+serial）、S-61（promote
    无闭包共享骨架 + `finish_promotion`）。
  - **Result**: 2/2 done；main agent 验收 grep 通过并修复 1 处顺带未用变量；
    compile withheld → run06 PASS（exit 0）；已随 user 指令 amend。
  - **Artifacts**: `subagent-tasks/code-structure-cleanup/
    task_creator_overlayfs_g8_direction_step2_20260818_dispatch.md`；收据
    `components/code-structure-cleanup/g8_direction_step2_20260818/`。

- **`task_reviewer_overlayfs_g10_keep_relocation_20260818`** — **ACCEPTED 2026-08-18**
  - **Kind**: bounded Reviewer wave（G10 KEEP 归位研讨 + 方向项 helper/method
    审计；opencode-go + deepseek-v4-flash，只读）。
  - **Result**: 报告 121/121 闭合（KEEP 119；需小调整 1 = `OPAQUE_MARKER_VALUE`；
    可移动 1 = `workdir_workspace_path` co-location，建议保守 KEEP）；方向项
    实体审计：`run_recipe` 不移 dir、`workdir_temp_name` 不纯随机简化、
    `finish_promotion` 合理；两份收据 census 无遗漏；0 `.rs` 改动。
  - **Next**: **P1 已执行**（`OPAQUE_MARKER_VALUE` → `pub(super)`，1 行；
    run07 PASS，当前 KEEP=120）；P2 未执行（报告建议保守 KEEP）。
  - **Artifacts**: packet `subagent-tasks/code-structure-cleanup/
    task_reviewer_overlayfs_g10_keep_relocation_20260818_dispatch.md`；报告
    `components/code-structure-cleanup/visibility_keep_relocation_20260818/
    task_reviewer_overlayfs_g10_keep_relocation_20260818_report.md`。

- **`parent_copyup_state_batches_20260827`** — **ACCEPTED (main-agent structural
  acceptance; runtime gates deferred)**
  - **Kind**: user-directed four-batch internal-restructure implementation wave
    (slicing: `PASS_SLICING.md` `parent_copyup_batches_slicing_20260827`;
    one Creator per batch, serial, Direct Spawn Lane).
  - **Basis**: live handoff `20260826-140940-parent-copyup-state-design` +
    designdoc proposal-final + 2026-08-27 study Q1/Q2 verdicts.
  - **Result**: pass_46 (Layer strong-mount), pass_47 (parent pointer +
    CopyUpState + ProjectionBinding), pass_48 (readdir `..` via parent +
    path-backed RealObject endpoint), pass_49 (field ordering + Q1 data-path
    redundancy merge) all exact-diff accepted. Final state: `OverlayInode`
    carries `parent: RwMutex<Weak<..>>` + `copyup: Mutex<CopyUpState>` with a
    self-parent mount root; copy-up coordination is coordinate-at-projection
    (`Done | Outstanding(CopyUpTarget{need_repair})`); readdir `..` reads the
    parent pointer directly (`.. == .` at root); every `RealObject` is
    path-backed with infallible `real_inode()`.
  - **Artifacts**: receipts under
    `components/parent-copyup-state-design/`; per-batch diff snapshots under
    `components/parent-copyup-state-design/run_evidence/pass_{46,47,48,49}/`.
  - **Deferred gates**: no compile/lint/xfstests run this session (user
    directive; container closed). First compile belongs to a later Checker;
    working tree left uncommitted pending user instruction.
  - **Static gate CLOSED (2026-08-27, container opened)**: Checker task
    `task_checker_parent_copyup_compile_lint_20260827` — three `make check`
    rounds; run01 FAIL at doc/rustfmt front gates, run02 FAIL with 4 clippy
    errors, run03 **PASS exit 0** (full pipeline green). All diagnostics were
    mechanical; main-agent fixes: 4 markdown trailing-whitespace lines,
    9 rustfmt hunks, 3 struct-constructor field reorderings, deletion of two
    zero-caller orphan methods (`IdentityPolicy::is_all_layers_same_fs`,
    `is_directory_projection_deterministic` — main-agent adjudication over the
    Checker's conservative design tag, evidence in PASS_SLICING entry), one
    stale doc clause. Runtime xfstests revalidation remains deferred pending
    user instruction.

- **`parent_copyup_wave_slicing_20260830`** — **EXECUTED & CLOSED 2026-08-30**:
  five passes (50a/50/51/55 + wave-close make check) all Creator→Reviewer
  closed and committed (d05b1bea1, 462da89ad, 47edb8d4e, ff0cdc1fe,
  14c404b45); `make check` GREEN (run04 exit 0, 16 mechanical fixes, zero
  semantic findings); runtime xfstests NOT run (user directive) — deferred
  to a future meso-integration Checker. One Creator-rework round on pass_55
  (present_xattr_names strip branch; same-Creator fix, same-Reviewer
  re-verify PASS). pass_51 carried the user-adjudicated deviation: workdir
  rides the upper clone view (proposal §3 literal amended); layer.rs origin
  arithmetic aligned to the uniform index rule.
  - **Kind**: user-directed wave opening for handoff backlog (a)–(e) + xattr
    gap list; slicing in `PASS_SLICING.md` (`parent_copyup_wave_slicing_20260830`).
  - **D1 `task_designer_rekey_convergence_20260830` — ACCEPTED** (structural
    acceptance + code spot-checks of four load-bearing claims). Verdicts:
    same-object live-different-instance displacement is unreachable in
    audited code (rename→rekey fully under the publication-parent DIR lock;
    all 7 fresh-truth scan entries hold the scanned parent's DIR lock) —
    backlog (e) is contract-driven robustness, not a reachable-bug fix;
    the real fragility is the post-rename failure classes (upgrade-race Err,
    `upper_real_object` re-resolution, post-rename `store_lower_id` +
    `need_repair`). Convergence semantics: (B) occupant yields, winner holds
    the key; rekey displacement predicate → diagnostic classification;
    commit liveness recheck → three-arm same-object predicate (capability-
    less arm-3 gap recorded). Algorithm frozen: infallible `publish_rekey`
    (pin-parameterized, unconditional publication) + reorder
    (`upper.call_once` before registration) + upper_real derived from the
    published path; `is_same_object` KEEP; lock nest
    `CUL -> DIR -> InodeCache.entries.write()`, zero new edges. D2 takes the
    report as input-of-record.
  - **xattr route change (user 2026-08-30)**: incremental gap-fix passes
    pass_52/53 superseded by a whole-module refactor per proposal §13
    (`task_designer_xattr_refactor_20260830`, dispatched; closes E1–E3/
    U1–U3/P1–P3/M1 by design and resolves all 15 xattr.rs orphans).
    pass_54 (R1/M2, syscall layer) unchanged. pass_50a (whiteout orphan
    moves, user-approved) added before pass_50.
  - **Order (amended 2)**: pass_50a → pass_50 → pass_51 → xattr_refactor
    → wave-close gates (`make check` + full xfstests table + Reviewer over
    cumulative diff). No parallel branch: pass_54 (R1/M2 syscall rules)
    withdrawn from the wave per user decision 2026-08-30 ("尽量不动 VFS";
    R1/M2 deferred alongside G1 and f, design anchors retained). All
    production changes confined to the overlayfs directory. The xattr
    refactor's 4 open questions await user adjudication before its packet
    dispatch.

- **`orphan_helper_audit_20260830`** — **ACCEPTED 2026-08-30 (structural +
  independent recount; execution pending user disposition)**
  - **Kind**: bounded read-only Reviewer audit (user-directed, run in
    parallel with D1): full-tree census of orphan helpers (top-level free
    fns outside impl blocks) with per-fn verdicts — KEEP (pure util) /
    MOVE INTO impl (same module) / CROSS-MODULE PROPOSAL (adjudication
    only). User criterion of record in the packet; structure frozen, so
    within-module relocation is the default prescription.
  - **Result**: 35 orphan fns across 8 of 28 live files (xattr.rs 15 worst;
    mod.rs 7; whiteout.rs 4; permission.rs 3; layer_parts/lookup 2 each;
    identity/remove 1 each). Verdicts: **KEEP 29 / MOVE 5 / CROSS 1**.
    MOVE: `copy_eligible_xattrs`/`set_impure_marker`/`clear_impure_marker`
    → `impl OverlayInode` (xattr.rs); `cleanup_upper_whiteouts`/
    `unlink_rechecked_whiteouts` → `impl OverlayFs` (whiteout.rs) — all
    Self-free associated fns, pure relocations. CROSS: `init` (mod.rs:123,
    global registry side effect) → `impl OverlayFsType` proposal, with the
    "lifecycle entry hook" framing recorded as the counter-consideration.
    Main-agent verification: independent column-0 recount matched 35/35
    per-file counts; MOVE target impl blocks confirmed at cited lines;
    call-site counts reconciled (extra greps were import lines).
  - **Disposition (user-adjudicated 2026-08-30)**: whiteout MOVE 4/5
    (`cleanup_upper_whiteouts`, `unlink_rechecked_whiteouts` → `impl
    OverlayFs`) approved — executing as `pass_50a_whiteout_orphan_moves`
    (recorded in PASS_SLICING). `init` CROSS proposal REJECTED — stays in
    place under the lifecycle-entry-hook framing. The three xattr.rs MOVEs
    (`copy_eligible_xattrs`/`set_impure_marker`/`clear_impure_marker`) are
    superseded into the xattr whole-module refactor (not executed
    piecemeal); the refactor resolves all 15 xattr.rs orphans by design.
    Cohesion predicates do not relocate.
  - **Artifacts**: packet `subagent-tasks/orphan-helper-audit-20260830/`;
    report `components/orphan-helper-audit-20260830/
    orphan_helper_audit_20260830.md`.

- **`comment_topdoc_20260902`** — **EXECUTED & gate-accepted 2026-09-04
  (comment-only top-doc round closed per user directive)**
  - **Kind**: comment-only documentation wave (top-level `//!` module docs;
    slicing in `PASS_SLICING.md` `comment_topdoc_20260902`). T1 front-door
    draft (Designer, accepted) + T2 three read-only audit lanes (A fs 14 /
    B inode 16 / C dir+copyup 10 findings), dispatched 2026-09-02.
  - **User-directed close (2026-09-04)**: this round touches ONLY `//!`
    module docs (`///` and inline comments reserved by the user for a later
    round); comment-only changes receive no Reviewer wave and no compile —
    done when applied.
  - **Result**: main agent accepted all 40 T2 findings (anchors verified)
    with two P10 adjudication amendments (T1 vocabulary row →
    `per-inode transaction lock`; C-08 → `copy-up coordinate`); four parallel
    Creators applied T1 (mod.rs 125-line crate doc + fs_type/layer/real
    rewrites) and all 40 findings across 18 files — 22 `.rs` files total,
    comment-only (non-comment diff = 0; lock-abbreviation / stale-term /
    inline-Linux grep = 0 in `//!` scope; the pre-existing §13 rustfmt
    test-block hunks in identity.rs/xattr.rs untouched). Two ktest module-doc
    `test-assets §` citations deferred to the later round per T1 Annex A.
  - **Artifacts**: draft/audits/receipts under
    `components/comment-topdoc-20260902/`; packets under
    `subagent-tasks/comment-topdoc-20260902/`.
  - **Deferred**: per-pass commit awaits user instruction (changes
    uncommitted); the `///`/inline-comment full-tree re-audit round is
    user-scheduled.

- **`comment_inline_20260904`** — **EXECUTED & gate-accepted 2026-09-04
  (harsh `///` + inline `//` adjudication round closed same day)**
  - **Kind**: comment-only adjudication + execution wave (user rulings of
    record: one sentence per `///` / two per inline `//` as upper bounds
    only; **default disposition = DELETE; survival is the exception**;
    harsher than the ext2 baseline, virtiofs-calibrated; `//!` module docs
    out of scope). Slicing in `PASS_SLICING.md` `comment_inline_20260904`.
  - **Audit phase**: six parallel read-only Reviewers (door / fsmount /
    options / inodecore / identity_xattr / dircopy) — census 589 blocks /
    ~1725 lines, per-file grep accounting closed; verdicts DELETE ~86%,
    KEEP 101 items (each with a cap-compliant exact replacement),
    MOVE 2 (both main-agent-accepted: options.rs params.c References
    consolidation + override_creds fail-fast rationale to module doc).
  - **Execution phase**: five parallel Creators, disjoint write-sets,
    mechanical application — 588/588 items applied, 0 stopped. One audit
    accounting slip (8 uncovered `rename_upper` doc lines) supplemented by
    main-agent adjudication (1 sentence kept). Gates (main-agent mechanical;
    no Reviewer/compile per standing user ruling): comment/blank-only diff
    PASS; forbidden-token residual grep = 0; survivors 248 lines tree-wide
    (~86% reduction).
  - **Artifacts**: criteria/packets under
    `subagent-tasks/comment-inline-20260904/`; six audit reports + five
    exec receipts + four review reports under
    `components/comment-inline-20260904/`.
  - **Review phase (user-directed)**: four read-only Reviewers, five-gate
    protocol (traceability / KEEP fidelity / boundary / residual tokens /
    independently re-derived coverage) — 4/4 PASS, zero mismatches, zero
    execution defects. Main-agent adjudication of two cross-lane
    observations (carrier docs deleted by a sibling lane): **code-as-spec
    accepted** — marker-read semantics carried by the `has_marker` match
    arms, real-object invariant by the `debug_assert` in
    `RealObjectStack::new`. Round **CLOSED (review-accepted)**.
  - **Deferred**: per-pass commit awaits user instruction. Side finding
    adjudicated by the user (2026-09-04): `UuidMode::Null` vs `Off`
    behavioral indistinguishability becomes its own future round (code
    placement and redundancy handling), not a comment-line item.

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

- **`placement_redundancy_20260904`** — **EXECUTED & accepted 2026-09-04
  (enum-variant redundancy round, two user-adjudicated items)**
  - **Kind**: bounded round (read-only enum audit → bounded Designer
    revision → single Creator pass with compile preflight). Slicing in
    `PASS_SLICING.md` `placement_redundancy_20260904`.
  - **Result**: tree-wide enum census (19 enums / 56 variants, zero dead;
    2 inseparable groups). User rulings: keep `UuidMode::Null` with a
    one-sentence upstream-anchor doc (Off vs Null diverge only in
    `ovl_origin_uuid`/`ovl_uuid_match` origin-fh validation — unimplemented
    locally; Null is upstream's degrade target); delete
    `NegativeLookup::HiddenByOpaque` into `Absent` (no upstream operation
    can distinguish an opaque-parent miss from absence). Applied verbatim;
    compile gate `cargo osdk check -p aster-core` exit 0; one enum variant
    removed (56→55), zero new entities. Main-agent exact-diff acceptance.
  - **Artifacts**: audit/designer/exec under
    `components/placement-redundancy-20260904/`; packets under
    `subagent-tasks/placement-redundancy-20260904/`.
  - **Deferred**: per-pass commit awaits user instruction.
