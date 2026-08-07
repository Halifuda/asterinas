<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: 2026-08-05 Wave7 xfstests Sequencing

**Date / Time:** 2026-08-05
**Status:** `Wave7 运行中（2026-08-07 单纯用例批次完成）。已完成 22 例
single-purpose 用例：8 PASS（009/002/007/016/003/006/011/039）、6 FAIL
（024/010/013/012/026/014）、8 NOTRUN（035/040/027 chattr；
004/008/015/025 fsgqa；023 chacl）。综合/stress 用例按用户指示暂不测
（031/020/029/078/077/038/041/001/021/019）。早期过程：001 NOTRUN（scratch
空间）、表格清洗重排、6 例不可调度（loop/index）。下一步：修复 FAIL
根因或按用户指示推进后续批次。`

## 1. Global State Pointer

- **Current Active Wave / Pass:** Wave7 xfstests sequencing
  (`wave7_xfstests_sequencing_20260805`), started (case 1 run 1 complete;
  table cleaned and re-sorted 2026-08-07).
- **Predecessor:** Wave6 documentation review is closed. Its static chain
  (workspace Clippy, `cargo fmt --check`, and `make check`) passed; this does
  not constitute runtime xfstests evidence or final acceptance.
- **Blueprint Updates Made:** No. This handoff is the user-requested planning
  record only; `SYSTEM_BLUEPRINT.md` and `PASS_SLICING.md` retain their
  accepted Wave6 state until Wave7 is expressly started.
- **Scope source:** Stage D makes all P0/P1 and `P2-01 xino` mandatory, and
  the later accepted scope amendment also includes `P2-11 UUID modes`.
  The six accepted Designer validation contracts remain the external-evidence
  source. Source reconciliation (2026-08-07) shows no packaged case observes
  `P2-11` (Section 3); xfstests is many-to-many evidence, not a
  one-test-per-micro claim.

## 2. Ordered Current-Scope xfstests Obligation

Run the following cases one at a time, in this order. Before attributing any
result, the Checker must confirm the actual case setup and asserted theme from
the upstream suite source, preserve the result file and guest log, and record
`PASS`, `FAIL`, or `NOTRUN` per case. A failing case follows the normal Checker
evidence and repair-loop protocol; this table does not authorize a repair or
rerun.

**2026-08-07 reconciliation:** every "Current-scope purpose" below was
re-derived from the upstream suite source (`tests/overlay/<case>`), replacing
the previous mis-attributed themes. Sort key = functional foundation (mount /
read-only fundamentals first; then copy-up and file views; then whiteout and
namespace semantics; then xattr/metadata; then permission/credential/stacked
behavior; then cache invalidation and xino) and, within a tier, test-unit
simplicity (`S1` = scratch + mount only; `S2` = one extra standard dependency;
`S3` = user/group/unshare/relatime or multi-scenario setup; `H` = heavy data
or concurrency). Cases needing the deferred `index=on` feature or a loop/XFS
backing lane are listed after the main table and are not schedulable now.

### 2.1 Schedulable cases (32), in execution order

| Order | Case | Upstream theme (from suite source) | Foundation tier | Simplicity |
| ---: | :--- | :--- | :--- | :--- |
| 1 | `overlay/035` | Read-only mount cases: no `upperdir`; immutable workdir forces read-only mount; no remount to rw. | Mount / read-only fundamentals | S2 (`chattr i`) |
| 2 | `overlay/024` | `workdir/work` leftovers must be cleaned at mount; mount must stay rw. | Mount / read-only fundamentals | S1 |
| 3 | `overlay/009` | `default_permissions` mount must not leak dentries (mount + read + unmount). | Mount / option behavior | S1 |
| 4 | `overlay/002` | fsync on a merged-directory file (write through overlay then fsync) must not crash. | Basic file I/O | S1 |
| 5 | `overlay/007` | getcwd must not fail after an unsuccessful rmdir on the test dir. | Basic path/name resolution | S1 |
| 6 | `overlay/016` | ro/rw fd consistency: read fd must not see stale data after write-through copy-up. | Copy-up + file views | S1 |
| 7 | `overlay/013` | truncate of running executables from lower/upper must fail `ETXTBSY`. | Copy-up + file views | S2 (helper `t_truncate_self`) |
| 8 | `overlay/004` | Copy-up triggered by mode-bit change; unprivileged user cannot trigger it. | Copy-up + file views | S2 (`_require_user`) |
| 9 | `overlay/003` | Basic whiteout visibility sanity when upper fs lacks `d_type` (mount gate). | Whiteout / visibility | S1 |
| 10 | `overlay/010` | rmdir of a lower dir containing whiteouts must not crash. | Whiteout / visibility | S1 |
| 11 | `overlay/012` | Stale upper dentry (removed from upper behind the overlay) must not warn/oops on remove. | Whiteout / stale state | S1 |
| 12 | `overlay/006` | Rename from lower to upper must not leave a visible whiteout. | Whiteout / visibility | S1 |
| 13 | `overlay/031` | Invalid whiteouts after lowerdir change + remount must not be exposed; rmdir must work. | Whiteout / remount | S2 (multi-mount) |
| 14 | `overlay/008` | Create over a whiteout as another user: uid/gid must be the creator's, not the mounter's. | Whiteout / permissions | S2 (`_require_user`) |
| 15 | `overlay/015` | SGID bit inheritance over a whiteout. | Whiteout / permissions | S2 (`_require_user` + `_require_group`) |
| 16 | `overlay/011` | `trusted.overlay.opaque` on an intermediate lower must be hidden from users. | Xattr / metadata | S2 (`trusted` attrs) |
| 17 | `overlay/026` | Xattr prefix filter: `trusted.overlay.*` rejected, `trusted.overlayxxx` allowed. | Xattr / metadata | S1 |
| 18 | `overlay/023` | Workdir must not inherit default POSIX ACLs from its parent. | Xattr / metadata | S2 (`_require_acls`) |
| 19 | `overlay/014` | Copy-up of an opaque-xattr lower dir must not copy the opaque flag. | Xattr / copy-up | S1 |
| 20 | `overlay/020` | Copy-up/namespace/cred: mounter superblock creds must be used (uses `unshare`). | Credential / namespace | S3 (userns) |
| 21 | `overlay/025` | POSIX ACLs on tmpfs-backed lower/upper layers must be readable. | ACL / layered backend | S3 (`tmpfs` extra fs + user) |
| 22 | `overlay/029` | Stacked overlay (overlay mounted as lower of another overlay) must resolve via `d_real`. | Stacked layers | S2 (nested mount) |
| 23 | `overlay/040` | Writing ioctl on a lower file must return `EPERM`, not modify the lower file. | Metadata / fileattr | S2 (`chattr i`) |
| 24 | `overlay/027` | Immutable upper file must stay untouchable through the overlay. | Metadata / fileattr | S2 (`chattr i`) |
| 25 | `overlay/078` | Copy-up of lower file attributes (immutable/append/sync/noatime) across copy-up + mount cycle. | Metadata / fileattr | S3 (`chattr ASai`, `syncfs`, scratch shutdown) |
| 26 | `overlay/039` | relatime updates for directories in the upper layer. | Metadata / time | S2 (`relatime`) |
| 27 | `overlay/077` | Readdir cache invalidation on changes to a dir with origin; stale entries skipped. | Readdir cache | S1 |
| 28 | `overlay/038` | Consistent `d_ino` for same-filesystem setup. | xino (`P2-01`) | S2 (`t_dir_type` + `trusted` attrs) |
| 29 | `overlay/041` | Consistent `d_ino` for non-same-filesystem setup (variant of 038). | xino (`P2-01`) | S3 (non-samefs setup + `t_dir_type`) |
| 30 | `overlay/001` | Copy-up of lower files <, =, > 4 GiB (`O_LARGEFILE` regression); needs ≥8 GiB free on scratch. | Copy-up (large file) | H (≥8 GiB scratch) |
| 31 | `overlay/021` | Concurrent copy-up of many files (copy-up bombs, 4 parallel teams); needs ~16 GiB scratch. | Copy-up (concurrency) | H (≥16 GiB scratch) |
| 32 | `overlay/019` | fsstress concurrently on lower and top dirs. | Stress | H (stress) |

### 2.2 Not schedulable in the current scope (6)

| Case | Upstream theme (from suite source) | Why not schedulable now |
| :--- | :--- | :--- |
| `overlay/005` | Copy-up error path memleak → panic on unmount; uses loop devices + XFS backing. | `_require_loop`; harness lane is ext2 images, no loop/XFS backing lane; scope decision needed. |
| `overlay/032` | Concurrent copy-up of lower hardlinks. | `_require_scratch_feature index`; `index=on` is deferred P3 behavior. |
| `overlay/033` | nlink accounting for overlay hardlinks. | `_require_scratch_feature index`; deferred. |
| `overlay/034` | nlink with unaccounted lower hardlinks. | `_require_scratch_feature index`; deferred. |
| `overlay/037` | Mount error cases with `index=on` (ESTALE on mismatch). | `_require_scratch_feature index`; deferred. |
| `overlay/042` | Lookup of non-indexed upper after offline lower-hardlink creation. | `_require_scratch_feature index`; deferred. |

The order is intentional: mount and read-only fundamentals precede copy-up and
file views; whiteout and namespace semantics precede xattr/metadata and
credential/stacked behavior; cache invalidation follows the readdir/xino
baselines it depends on; heavy data and stress cases are last because they
need large scratch images and long runtime.

## 3. Explicit Scope Exclusions

- Do not schedule `overlay/017`, `043`, or `057`: they require deferred
  `P2-02 redirect_dir` behavior, even where they incidentally observe a basic
  stat or xino path.
- Do not schedule `overlay/018`, `028`, or `044` as required whole-case
  passes: their lower-hardlink/origin/nlink assertions need deferred
  association/index or `P2-07` behavior. The current basic contract permits
  only the non-index upper-authoritative link observation, not treating a
  whole mixed-scope case as passed.
- Do not schedule `overlay/030`, `075`, or `076` (deferred `P2-06`
  fileattr), `overlay/083`, `084`, or `109` (deferred userxattr/escaping or
  unavailable packaged lane), or any P3 index, NFS export, metacopy,
  data-only, fs-verity, trap, nested-overlay, or cleanup case.
- Do not schedule `overlay/005` (requires loop devices and an XFS backing
  lane not present in the packaged ext2-image harness), nor `overlay/032`,
  `033`, `034`, `037`, or `042` (all require `_require_scratch_feature index`,
  i.e. deferred `index=on` behavior). See Section 2.2.
- `overlay/029` stays schedulable in Section 2.1: its stacked-overlay setup
  exercises the P0/P1 `d_real` path with only `_require_scratch`, which the
  prior generic "nested-overlay" exclusion was not aimed at; the P3 nested
  overlay/cleanup class remains excluded.
- Do not schedule `overlay/067` or `068` as UUID tests, and do not treat
  `overlay/078` as a UUID observation: source reconciliation shows 078 is a
  fileattr copy-up test (`copyup perms shutdown`) requiring `chattr ASai`,
  `syncfs`, and scratch shutdown, and it now sits with the deferred `P2-06`
  fileattr class. The staged inventory's dedicated `overlay/081` UUID row is
  absent from the packaged `full.list`. Until an explicitly authorized
  source-level reconciliation changes that mapping, no packaged case is a
  current-scope UUID observation.

## 4. Thread Activity Log

- **Logic-bug repair wave start (2026-08-07, user-directed):** user
  authorized Designer + Creator repair of the five Wave7 FAIL groups and a
  main-agent compile check after implementation (no experiment restart).
  Pass-slicing decision `wave7_logic_bug_repair_20260807` recorded in
  `PASS_SLICING.md`; upstream sources for `overlay/010`, `012`, `013`,
  `014`, `024`, `026` preserved under
  `components/wave7-xfstests-sequencing/run_evidence/upstream_sources/`.
  Designer packet
  `subagent-tasks/wave7_logic_bug_repair/task_designer_wave7_logic_bug_repair_20260807_dispatch.md`
  created; Designer dispatched (or main-agent fallback per the multi-agent V2
  delivery note) to produce the bounded repair addendum under
  `components/wave7_logic_bug_repair/`.
- **Designer acceptance (2026-08-07):** `task_designer_wave7_logic_bug_repair_20260807`
  ACCEPTED structurally. The bounded repair addendum
  (`components/wave7_logic_bug_repair/wave7_logic_bug_repair_designer_spec.md`
  + `_designer_validation.md`) freezes all five objectives: O1 workdir
  residue cleanup (`prepare_workdir` + `remove_work_entries` +
  `WORK_RESIDUE_NAME`, no `ENOTEMPTY` on root residue), O2 readdir opaque
  layer barrier (merge stops after the first opaque layer), O3 `ETXTBSY`
  (VFS `Inode` default-method seam; `exec_write_deny_count` on
  `OverlayInode`; `resize_impl` check before copy-up; deny at exec/init/fork
  and release at `ProcessVm` drop), O4 getxattr `EOPNOTSUPP` for non-`Public`
  names, O5 `translate_stale_upper_enoent` on the three physical-upper
  `ENOENT` sites. No topology/lock change, no ktest surface, no pass slicing.
  Creator packet
  `subagent-tasks/wave7_logic_bug_repair/pass_28_wave7_logic_bug_repair_creator_dispatch.md`
  created; Creator dispatch next (command-free; main agent then verifies the
  target-specific `cargo check` per user direction).
- **Creator pass `pass_28_wave7_logic_bug_repair` (2026-08-07): ACCEPTED with
  one main-agent mechanical continuation.** The Creator (codex-CLI fallback)
  implemented all five objectives per the accepted Designer spec and filed
  `components/wave7_logic_bug_repair/pass_28_wave7_logic_bug_repair_creator.md`
  with the full entity census (WORK_RESIDUE_NAME, remove_work_entries,
  exec_write_deny_count, two Inode trait defaults, two OverlayInode
  overrides, is_write_denied, ProcessVm Drop, translate_stale_upper_enoent).
  The Creator reported one O3 placement deviation (check on the `resize`
  entry instead of `resize_impl` because `copyup/mod.rs` was outside the
  packet write-set) and one mandatory out-of-write-set initializer edit in
  `projection/mod.rs`. The main agent then applied a same-pass mechanical
  continuation to restore the frozen ordering exactly: moved the ETXTBSY
  gate into `copyup/mod.rs::resize_impl` (after the EROFS and MAY_WRITE
  gates, before copy-up), widened `is_write_denied` to
  `pub(in crate::fs::fs_impls::overlayfs)` for the sibling call, widened the
  `exec_write_deny_count` field to `pub(super)` for the inode-cache
  constructor, removed the now-orphaned `DirentCounter` utility (its only
  active consumer was the deleted `is_workdir_empty`), dropped the redundant
  `Inode` imports in `init_proc.rs`/`process_vm/mod.rs`, underscored the
  unused `Vec<String>` visitor params, and switched the deprecated
  `fetch_update` to `try_update`. `cargo fmt --check` on the changed files
  passes.
- **Compile verification (2026-08-07, user-directed, main-agent run):**
  `docker exec -w /root/asterinas codex-asterinas-dev bash -lc 'cd
  /root/asterinas/kernel && cargo check -p aster-kernel --target
  x86_64-unknown-none'` exits 0 with exactly one pre-existing warning
  (`MountPolicy::uuid_mode` dead field in `mount/policy.rs`, untouched by
  this wave). No QEMU/xfstests run was started (user: do not restart the
  experiment). Full `make kernel`/`make check`/xfstests remain unscheduled.
- **VFS revert (2026-08-07, user-directed):** the O3 `ETXTBSY` mechanism was
  removed in full — `Inode::deny_write_access`/`allow_write_access` defaults
  (`fs/vfs/fs_apis/inode.rs`), the `OverlayInode` count field/overrides/
  helper (`projection/inode.rs`), the inode-cache initializer
  (`projection/mod.rs`), the `resize_impl` gate (`copyup/mod.rs`), and the
  exec/fork/drop lifecycle (`process/execve.rs`,
  `process/process/init_proc.rs`, `process/process_vm/mod.rs`). All seven
  files are byte-identical to HEAD. Interface-breaking VFS modifications are
  refused for this wave; `overlay/013`'s `ETXTBSY` remains the documented
  §19c divergence pending a redesign with no VFS interface change.
- **Workdir workspace Designer audit (2026-08-07): ACCEPTED.** User-directed
  revision `task_designer_wave7_workdir_workspace_audit_20260807`:
  `<workdir>/work` becomes the actual staging workspace (Linux
  `OVL_WORKDIR_NAME`/`ofs->workdir` parity). Audit disposed all six usage
  site groups (workspace resolver, mount preparation, capability probes,
  claim surface, staging consumers, carriers) and froze the Rust surface:
  pinned `workdir_workspace: Option<Arc<dyn Inode>>` on
  `UpperWorkdirClaim` + `workdir_workspace()` accessor; `prepare_workdir`
  creates/pins the workspace (residue-clean + recreate, no `ENOTEMPTY`);
  mount order prepare (step 7) → probes against the workspace (step 8) →
  UUID persist (step 9); `workdir_root` resolves the workspace; probes
  renamed `workdir_inode` → `workspace_inode` (logic unchanged);
  `workdir_inode()` kept as the claim surface with `#[expect(dead_code)]`.
  Artifacts:
  `components/wave7_logic_bug_repair/wave7_workdir_workspace_designer_spec.md`
  + `_designer_validation.md`.
- **Workdir workspace Creator pass `pass_29_wave7_workdir_workspace`
  (2026-08-07): ACCEPTED.** Implemented the frozen surface with no
  deviations; report
  `components/wave7_logic_bug_repair/pass_29_wave7_workdir_workspace_creator.md`.
  Main-agent compile verification reran after acceptance:
  `cargo check -p aster-kernel --target x86_64-unknown-none` exits 0 (one
  pre-existing `MountPolicy::uuid_mode` warning); `cargo fmt --check` clean
  on all changed files. No QEMU/xfstests run.
- **Stale-upper TODO annotation (2026-08-07, user-directed):** added a
  `TODO(stale-upper)` doc annotation on `translate_stale_upper_enoent`
  (`dir/remove.rs`) recording that the post-operation `ENOENT`→`ESTALE`
  translation is an indirect approximation; the faithful approach is
  VFS-level dentry verification (fresh upper lookup vs cached upper before
  the physical op, Linux `ovl_matches_upper`), deferred because it would
  require a breaking VFS interface/behavior change that this wave
  intentionally avoids. Comment-only edit; `cargo check` still exits 0.
- **Commit `3299cc8b5` (2026-08-07):** `Fix overlayfs bugs found by simple
  xfstests cases` — 13 Rust files (workdir workspace, readdir opaque
  barrier, xattr `EOPNOTSUPP`, stale-upper `ESTALE` + TODO, `DirentCounter`
  removal). `.agents` records remain uncommitted per protocol.
- **Fixed-case re-run batch (2026-08-07, user-directed; main agent executed
  the Checker lane directly — subagent dispatch unavailable):** serial
  single-case runs of `overlay/024`, `010`, `014`, `026`, `012` on commit
  `3299cc8b5`. Result: `010` **PASS**; `024` **FAIL** (cleanup happens on
  disk but the base mount's stale `DentryChildren` cache still reports
  `work/foo` — prepare uses raw inode ops, bypassing the VFS dentry layer);
  `014` **FAIL** (pre-existing **lowerdir ordering inversion**:
  `normalize_lower_ordering` reverses the colon-split single option, but
  Linux stacks the first path topmost; the whiteout `d` in `lower2/testdir`
  never hides `lower1`'s `d`; secondary: `lower2/testdir` lacks the persisted
  opaque marker); `026`/`012` **FAIL at setup** (`prepare_workdir` creates
  `<workdir>/work` with mode 0o000, so the harness `rm -rf` sweep cannot
  unlink the previous run's leftover workdir temps → `Permission denied`).
  Full report: `components/wave7-xfstests-sequencing/pass_36_wave7_fixed_cases_rerun_checker.md`;
  per-case logs under `run_evidence/<case>/`. Repair batch: (1) workspace
  dir mode 0o700/0o755, (2) lowerdir ordering fix, (3) prepare-time dentry
  invalidation via a non-breaking seam, (4) opaque-marker persistence triage.
- **Workdir mode fix requirement (2026-08-07, user-directed):** when the
  workspace-dir mode repair lands (`prepare_workdir` creating `<workdir>/work`
  with a usable mode instead of `InodeMode::empty()`), the code MUST carry a
  TODO comment explaining the Linux divergence: Linux `ovl_workdir_create`
  creates `work/` with mode 0o000 (`S_IFDIR|0`, clearing inherited bits) and
  relies on `generic_permission`'s directory special-case (CAP_DAC_OVERRIDE
  overrides all DACs for `S_ISDIR`, including the no-exec-bit case), whereas
  this kernel's `check_permission` applies the "exec override requires at
  least one exec bit" rule to directories too, so root cannot traverse or
  unlink inside a 0o000 directory. The workspace therefore uses a usable mode
  (0o700/0o755) instead of replicating 0o000.
- **Prepare-time dentry invalidation divergence requirement (2026-08-07,
  user-directed):** when the prepare-time dentry invalidation repair lands
  (reworking `prepare_workdir` cleanup onto the mount-time `Path` API), the
  cleanup site MUST carry an explicit TODO/comment stating the inconsistency
  with Linux behavior: the current raw-inode cleanup bypasses the VFS dentry
  layer, so the base mount's stale `DentryChildren` cache still reports the
  removed entries (e.g. `work/foo`), whereas Linux performs the workdir
  cleanup through the VFS/upper-fs dentry layer so the cached directory view
  stays coherent; the rework therefore routes cleanup through the mount-time
  `Path` API to update the base view's `DentryChildren` with zero VFS
  interface change.
- **Fixed-case rerun Designer addendum (2026-08-07, in-thread Designer
  execution; no subagent per user direction):**
  `task_designer_wave7_fixed_case_rerun_20260807` ACCEPTED structurally.
  Artifacts:
  `components/wave7_logic_bug_repair/wave7_fixed_case_rerun_designer_spec.md`
  + `_designer_validation.md`. Frozen surface: (R2-A) delete
  `normalize_lower_ordering` and make `lower_dirs`/`lowers` topmost-first
  with Linux parity (`options.rs` doc corrections only; `layers.rs` loop
  consumes the parsed list directly); (R2-B) new `WORKDIR_MODE =
  InodeMode::from_bits_truncate(0o700)` const with the frozen divergence
  TODO; (R2-C) `prepare_workdir(&mut self, workdir_path: &Path)` — the
  visible `work` name is removed/recreated through
  `Path::rmdir`/`Path::unlink`/`Path::new_fs_child` so the base view's
  `DentryChildren` is updated, `remove_work_entries` raw recursion retained
  (residue dentry is discarded wholesale; rationale frozen in the spec),
  divergence TODO at the cleanup site, zero VFS interface change;
  `build.rs` step 7 passes `&workdir_path`. No topology/lock change, no
  ktest surface, no pass slicing, no production `.rs` written. Out of scope
  per handoff: 014 opaque-marker/clear-empty triage and `overlay/013`
  `ETXTBSY`.
- **Fixed-case rerun Creator pass `pass_37_wave7_fixed_case_rerun`
  (2026-08-07, in-thread; no subagent per user direction): ACCEPTED.**
  Implemented the accepted Designer addendum exactly (no deviations):
  `normalize_lower_ordering` deleted and `lower_dirs`/`lowers` topmost-first
  (`options.rs`/`layers.rs` docs corrected); `WORKDIR_MODE` const (0o700)
  with the frozen divergence doc; `prepare_workdir(&mut self,
  workdir_path: &Path)` removing/recreating the visible `work` name through
  `Path::rmdir`/`Path::unlink`/`Path::new_fs_child` with
  `TODO(workdir-cleanup-vfs-parity)` and `TODO(workdir-mode)` comments;
  `build.rs` step 7 passes `&workdir_path`. `remove_work_entries` raw
  recursion retained per the frozen rationale. Report:
  `components/wave7_logic_bug_repair/pass_37_wave7_fixed_case_rerun_creator.md`.
  Pass-slicing record added to `PASS_SLICING.md`.
- **Fixed-case rerun compile verification (2026-08-07, user-directed,
  main-agent run):** `cargo check -p aster-kernel --target
  x86_64-unknown-none` exits 0 (only the pre-existing
  `MountPolicy::uuid_mode` dead-field warning, untouched); `cargo fmt
  --check` on the four changed files exits 0. No QEMU/xfstests run (user:
  wait for instruction).
- **Fixed-case rerun xfstests (2026-08-07, user-directed; individual runs,
  each case exactly once; images rebuilt fresh; 014 last):** all five cases
  **PASS** — `overlay/024` (workdir cleanup + base-view coherence),
  `overlay/010` (whiteout residue), `overlay/026` (setup hygiene + xattr),
  `overlay/012` (setup hygiene + stale-upper), `overlay/014` (lowerdir
  ordering parity: `lowerdir2:lower1` now stacks `lowerdir2` topmost; the
  whiteout `d` is hidden and the post-copy-up/remount merged readdir is
  clean). The TEST/SCRATCH images were deleted and freshly recreated (8 GiB
  ext2, new UUIDs; SHA-256 recorded in the report) to clear the previous 014
  workdir pollution; the reused-image path also validated the cross-run
  `_scratch_mkfs` cleanup (mode fix) with no `Permission denied`. Temporary
  single-case runlist `wave7-single.list` created per case and deleted after
  the batch. Report:
  `components/wave7-xfstests-sequencing/pass_38_wave7_fixed_case_rerun_checker.md`;
  per-case logs under `run_evidence/<case>/rerun_20260807/`.
- **Post-batch cleanup (2026-08-07, user-directed):** the rebuilt
  TEST/SCRATCH images were **deleted** after evidence capture (Docker-over-WSL
  VHDX host-space concern); the receipt retains only their size (8589934592
  bytes each) and SHA-256 (recorded in `pass_38`). The four fixed-case-rerun
  production files (`mount/{options,layers,claims,build}.rs`) were **amended
  into the latest commit**, and per explicit user direction the three tracked
  `.agents` records (`PASS_SLICING.md` and both live main-agent handoffs)
  were amended in as well (same message "Fix overlayfs bugs found by simple
  xfstests cases"). Historical references to `3299cc8b5`/`e396c55cb` in this
  handoff describe pre-final-amend states; see `git log -1` for the current
  HEAD hash.
- **`overlay/014` sequencing decision (2026-08-07, user-directed):** fix the
  lowerdir layer-order inversion FIRST
  (`normalize_lower_ordering`/`mount/options.rs`: for a single colon-joined
  `lowerdir=` option the first path is the topmost layer per Linux docs; the
  current reverse makes `lower1` topmost and defeats both per-name whiteout
  lookup and the readdir opaque barrier), then RE-RUN `overlay/014` to
  observe whether new issues surface (the readdir barrier itself is believed
  correct; the absent persisted opaque marker on the recreated dir and the
  mount-1 clear-empty failure are separate triage items to be revisited only
  after the ordering fix).
- **单纯用例批次（2026-08-07）：** 按用户指示顺序执行全部 single-purpose
  用例（22 例），综合/stress 用例未测。结果矩阵与逐例证据：
  `wave7_simple_cases_batch_summary.md`；日志按用例归档于
  `run_evidence/<case>/`。
  - PASS（8）：`009`（default_permissions）、`002`（fsync）、`007`
    （getcwd）、`016`（ro/rw fd）、`003`（whiteout sanity）、`006`
    （rename whiteout）、`011`（opaque xattr 隐藏）、`039`（relatime）。
  - FAIL（6）：
    - `024`/`010`：workdir 根空检查过严（`prepare_workdir` 返回
      `ENOTEMPTY`，Linux 应清理 `work/` 子目录残留）；
    - `013`：truncate 运行中二进制未返回 `ETXTBSY`；
    - `012`：stale upper dentry 删除返回 `ENOENT`（预期 `ESTALE`）；
    - `026`：`trusted.overlay.*` getxattr 返回 `ENODATA`（预期
      `EOPNOTSUPP`）；
    - `014`：readdir 合并缺 lower 层 opaque 屏障（明确缺口：
      `readdir_index.rs::readdir_sequence` 只对 upper 检查 opaque，lower
      层屏障未实现，lowerdir2 opaque 未挡住 lowerdir1 的 `d`；copy-up
      xattr 过滤本身正确）。clear-empty 持久化问题暂不讨论。
  - NOTRUN（8）：`035`/`040`/`027` chattr 缺失；`004`/`008`/`015`/`025`
    `fsgqa` 用户缺失；`023` `chacl` 缺失。
- **Run 4 result (2026-08-07, `run_001_20260807_1139`):** `overlay/002`
  (new order 4) — **PASS** (merged-directory write + fsync; `Passed all 1
  tests`). Evidence: `run_evidence/overlay002/`; report
  `pass_34_wave7_overlay002_checker.md`.
- **Run 3 result (2026-08-07, `run_001_20260807_1137`):** `overlay/009`
  (new order 3) via temporary single-case runlist `wave7-single.list`,
  `XFSTESTS_DISK_SIZE=8G`, reused images. Result: **PASS**
  (`default_permissions` mount + read + clean teardown; `Passed all 1
  tests`). First behavioral PASS of Wave7. Evidence:
  `.agents/components/wave7-xfstests-sequencing/run_evidence/overlay009/`;
  report `pass_33_wave7_overlay009_checker.md`.
- **Run 2 result (2026-08-07, `run_001_20260807_1136`):** `overlay/024`
  (new order 2) — **FAIL** (first behavioral failure). Guest mount failed:
  `fsconfig() failed: Directory not empty` (`ENOTEMPTY`). Root cause located:
  `UpperWorkdirClaim::prepare_workdir()` (`mount/claims.rs:298-303`) requires
  the workdir root to be empty, while Linux overlay semantics keep the
  internal work under `<workdir>/work` and clean residue there at mount
  (upstream `tests/overlay/024` pre-creates `work/foo` and expects it gone
  after mount). Repair batch recorded in
  `pass_32_wave7_overlay024_checker.md`; route to the meso-01 workdir owner.
- **Run 2 result (2026-08-07, `run_001_20260807_1130`):** `overlay/035`
  (new order 1) via temporary single-case runlist `wave7-single.list`,
  `XFSTESTS_DISK_SIZE=8G`, fresh 8 GiB ext2 TEST/SCRATCH images, RELEASE=1
  MEM=12G. Kernel/initramfs built; guest reached xfstests with
  `FSTYP -- overlay`. Result: `[not run] file system doesn't support
  chattr +i`. No panic, hang, or filesystem behavior failure. Upstream
  source confirms the theme (read-only mount cases; one scenario needs
  `chattr +i` on the workdir), so the handoff theme stands. Evidence and
  report under `.agents/components/wave7-xfstests-sequencing/`
  (`pass_31_wave7_overlay035_checker.md`, `run_evidence/`); temporary
  runlist deleted; no QEMU remains.
- **Capability gap (2026-08-07):** the `chattr +i` prerequisite failure
  exposes that the current kernel has no fileattr/`FS_IOC_*` implementation
  (`rg` in `kernel/src/fs/` found none). This gates `overlay/040`, `027`,
  and `078` later in the order; those rows will NOTRUN at the same gate until
  fileattr support exists or the scope decision changes.
- **Table reconciliation (2026-08-07, user-directed):** Cleaned and re-sorted
  the Section 2 obligation table after reading the upstream suite source for
  all 38 cases. Most prior "Current-scope purpose" rows were mis-attributed
  (e.g. `overlay/002` is an fsync crash regression, not lookup; `overlay/004`
  is mode-bit copy-up, not opaque/whiteout lookup; `overlay/005` is a
  copy-up-error memleak regression needing loop/XFS, not merged readdir;
  `overlay/007` is getcwd-after-failed-rmdir, not readdir/`d_ino`;
  `overlay/019` is fsstress, not stat/readdir consistency; `overlay/021` is
  concurrent copy-up bombs, not writable-mount setup; `overlay/032`-`034`,
  `037`, `042` are `index=on` hardlink/nlink tests, not rename/SGID/xino;
  `overlay/039`/`040` are atime/ioctl regressions, not mmap/fsync
  delegation; `overlay/078` is fileattr copy-up, not UUID observation).
  New order: 32 schedulable cases by foundation tier then simplicity, plus
  six classified not schedulable now (Section 2.2). Heavy/space cases
  (`overlay/001` ≥8 GiB, `overlay/021` ~16 GiB, `overlay/019` stress) moved
  to the tail.
- **Dispatch / execution note (2026-08-07):** Two Checker subagents were
  spawned with `fork_turns="none"` per protocol; neither received its task
  content correctly. Per explicit user direction, both were interrupted and
  the main agent executed the authorized Checker lane directly.
- **Run 1 result (2026-08-07, `run_001_20260807_1120`):** `overlay/001`
  via temporary single-case runlist `wave7-single.list`, `XFSTESTS_DISK_SIZE=8G`,
  fresh 8 GiB ext2 TEST/SCRATCH images, RELEASE=1 MEM=12G. Kernel/initramfs
  built; QEMU booted; guest reached xfstests with `FSTYP -- overlay`.
  Result: `[not run] This test requires at least 8GB free on
  /opt/xfstests/scratch to run` (scratch image free ≈ 7.85 GiB). No panic,
  hang, or filesystem behavior failure. Evidence preserved under
  `.agents/components/wave7-xfstests-sequencing/run_evidence/`; report at
  `.agents/components/wave7-xfstests-sequencing/pass_30_wave7_overlay001_checker.md`;
  temporary runlist deleted; no QEMU remains.
- **Theme discrepancy (escalation, 2026-08-07):** the guest upstream source
  (`tests/overlay/001`) proves the case is a copy-up test for files <, =,
  > 4 GiB (`ovl: use O_LARGEFILE in ovl_copy_up()` regression), gated by
  `_require_fs_space $OVL_BASE_SCRATCH_MNT $((4*1024*1024*2 + 8))`. The
  Wave7 ordering and the meso-01 contract row calling it "mount-option
  validation / root-stat baseline" are not supported by the suite source.
- **Wave7 start decision (2026-08-07):** User explicitly authorized starting
  the xfstests sequencing recorded in Section 2.
- **Dispatch (2026-08-07):** Checker packet
  `pass_30_wave7_overlay001_checker_dispatch.md` created under
  `.agents/subagent-tasks/wave7-xfstests-sequencing/` and dispatched for
  `overlay/001` only, with evidence destination
  `.agents/components/wave7-xfstests-sequencing/run_evidence/`. The packet
  authorizes the verified overlay xfstests lane (`$ovfs-checker`) with a
  temporary single-case runlist
  (`test/initramfs/src/conformance/xfstests/overlay/run_list/wave7-single.list`),
  a fresh 8 GiB TEST/SCRATCH image pair, theme confirmation from the upstream
  suite source, and the exact scope exclusions in Section 3.
- **User-directed mechanical deferred-expect cleanup (2026-08-06):** Removed
  the unused claim and UUID accessors from `mount/claims.rs` and
  `mount/policy.rs`: `InodeClaimGuard::token`,
  `UpperWorkdirClaim::{has_exclusive_claim,upper_inode,identity}`, and
  `MountPolicy::uuid_mode`; and removed the now-unreferenced
  `OverlayInuseSlot::is_claimed_by` query. `UpperWorkdirClaim` no longer
  duplicates the upper filesystem `Arc`; `selected_real_fs` now reads the
  canonical upper filesystem from `OverlayLayerStack`. This is ownership
  cleanup only: claim acquisition, release, lifecycle semantics, and
  upper/lower selection remain unchanged. The nine retained `dead_code`
  expectations are the two VFS
  unmount/shutdown seams, two VFS scoped-credential seams, and five VFS
  writer/freeze seams; each now carries a concrete TODO and VFS-specific
  reason. `cargo fmt --check` and `git diff --check` passed. The usual
  workspace Clippy command could not start because this host lacks
  `cargo-osdk`; a standalone `cargo check -p aster-kernel` with an isolated
  target directory instead failed in pre-existing host configuration before
  compiling the kernel crate, because OSDK-provided architecture dependencies
  were absent. No Wave7 runtime command ran.
- **Dispatches Sent:** None.
- **Commands Run:** None for Wave7.
- **Acceptance Outcomes:** None. This is a scheduling record, not an
  xfstests result or an integration acceptance.
- **Escalations / Deadlocks:** None. The UUID mapping discrepancy is recorded
  above as a pre-start reconciliation item, not a runtime failure.

## 5. Explicit Agent-Level Decisions

1. Wave7 is started per explicit user authorization. The first run
   (`overlay/001`) is complete with `NOTRUN` (scratch-space prerequisite), and
   the Section 2 table has been cleaned and re-sorted from the upstream suite
   source. No behavioral evidence, acceptance, repair routing, or
   `legacy_fs.rs` deletion exists.
2. Only complete xfstests cases whose intended current-scope behavior can be
   passed as a whole are ordered. Mixed cases with deferred required assertions
   are excluded rather than counted as partial success.
3. Six previously listed cases (`overlay/005`, `032`, `033`, `034`, `037`,
   `042`) are not schedulable in the current lane because they require a
   loop/XFS backing lane or the deferred `index=on` feature; they are recorded
   in Section 2.2 and excluded until a scope decision changes that.
4. Creating this handoff did not itself authorize test, harness,
   `legacy_fs.rs`, production, VFS, Designer, Creator, or Reviewer work. The
   separately user-directed mechanical cleanup is recorded in Section 4 and
   does not authorize Wave7 implementation or runtime work.
5. A future runtime packet must retain xfstests as the sole validation lane;
   no ktest or filesystem-local substitute is permitted.

## 6. Next Actions for the Next Thread (CRITICAL)

1. **The three repair items from the fixed-case re-run batch (`pass_36`)
   are IMPLEMENTED and compile-verified** by the ACCEPTED Creator pass
   `pass_37_wave7_fixed_case_rerun` against the frozen Designer addendum
   `wave7_fixed_case_rerun_designer_spec.md` (see §4):
   a. **Lowerdir layer order (014):** `normalize_lower_ordering` deleted;
      `lower_dirs`/`lowers` topmost-first (first option = topmost, Linux
      parity); stale "rightmost" docs corrected.
   b. **Workdir workspace mode (026/012 setup):** `<workdir>/work` created
      with `WORKDIR_MODE` (0o700) instead of `InodeMode::empty()`, with the
      Linux-DAC-divergence TODO.
   c. **Prepare-time dentry invalidation (024):** `prepare_workdir` cleans
      the visible `work` name through the mount-time `Path` API so the base
      view's `DentryChildren` is updated; zero VFS interface change;
      divergence TODO at the cleanup site.
2. **Fixed-case batch re-run COMPLETE (2026-08-07):** `overlay/024`, `010`,
   `026`, `012`, `014` all **PASS** (individual runs, each once, images
   rebuilt fresh, 014 last). The deferred 014 triage items (opaque-marker
   persistence, clear-empty temp residue) are NOT needed because 014 passes.
   Remaining Wave7 obligations (chattr-gated `035`/`040`/`027`, fsgqa-gated
   `004`/`008`/`015`/`025`, chacl-gated `023`, heavy `001`/`021`/`019`)
   await an explicit user scope decision; `overlay/013` `ETXTBSY` stays an
   unscheduled §19c divergence.
3. **`overlay/013` ETXTBSY:** redesign without any VFS interface change
   (current mechanism fully reverted); not scheduled.
4. **Boundaries:** no VFS interface-breaking changes; no QEMU/xfstests run
   without explicit user authorization; `.agents` records stay uncommitted.

## 7. Live File Discipline

- **This file is the live handoff for:** the active Wave7 xfstests tenure.
- **Update rule:** Update this file in place for every Wave7 start decision,
  dispatch, result, repair routing, acceptance, rejection, or escalation.
- **Supersedes / Replaces:**
  `20260804-wave6-documentation-lint_main_agent_handoff.md`, closed / handed
  over on 2026-08-05.
