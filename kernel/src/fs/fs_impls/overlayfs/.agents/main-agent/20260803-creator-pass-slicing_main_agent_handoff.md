<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: 2026-08-03 Creator Pass Slicing (Phase 4)

> **ENDED / SUPERSEDED (2026-08-04):** Wave5 static continuation 11 reached
> the user-directed documentation-only stopping point. The single live
> handoff is now
> `20260804-wave6-documentation-lint_main_agent_handoff.md`.

**Date / Time:** 2026-08-03; last updated 2026-08-04
**Status:** `Closed / handed over to Wave6 documentation lint. No further Wave5 scheduling belongs in this handoff.`

## 1. Global State Pointer

- **Current Active Wave / Pass:** Wave5 static entry — continuation 11 at the
  current dirty source state passed the prescribed target-specific cargo smoke
  and `make kernel` after `MountPolicy::assemble` switched to a call-local
  borrowed `OverlayMountOptions` and `BindingCache::entries` gained the two
  approved private aliases. Workspace Clippy then stopped with only nine
  user-deferred Wave6 documentation diagnostics; `too_many_arguments`,
  `type_complexity`, and `boxed_local` are absent. rustfmt and `make check`
  remain unscheduled until the Wave6 documentation cleanup is complete;
  neither runtime nor xfstests is scheduled. The legacy registry wiring is
  removed and `mount::OverlayFsType` is active. P1/P2 remain deferred. Phase 4
  (implementation) is **COMPLETE and ACCEPTED**; Phase 3 (Designer) closed
  with all 6 basic-wave contracts `Specified`; meso 07 is deferred-only (0/4).
  The prior tenure's handoff
  (`20260801-state-cleanup-designer-prep_main_agent_handoff.md`) is marked
  **ENDED / SUPERSEDED**; this file is the single live handoff.
- **Stable commit chain (codex/overlayfs-refactor):** `be0e574c5` baseline
  (legacy rename + records) → `e1613f12c` Wave 1 (`mount/`) → `77b0d4a49`
  Wave 2 (`projection/` + OverlayFs extensions) → `b9b9d6caf` Wave 3
  (shared-carrier seams) → `43a0747bc` Wave 4 (leaf mesos) → `7aabd029c`
  accepted pre-wave5 closure → `36c30ac33` Wave5 takeover, five accepted
  owner/interface repairs, bounded ownership-order repair, and claim
  visibility propagation. The title has no `WIP`; tracked board files remain
  intentionally uncommitted.
- **Blueprint Updates Made:** Yes (2026-08-03): Phase 3 closed; Phase 4 opened with the pass-slicing decision, then marked **Complete** with the pipeline index rows set to `Implemented` (Creator pass done; per-pass Checker `not scheduled` by workflow amendment; integration Checker + Reviewer pending). `PASS_SLICING.md` carries the `creator_pass_slicing_20260803` block.
- **Accepted baseline:** Phase 0-2 accepted; fresh Architect topology owns all 81 formal Micro IDs; Stage-D scope 57 `需要实现` / 24 `暂不实现` unchanged; meso 01-06 contracts `Specified`; meso 07 deferred-only. **The full 57-micro implementation set is now landed (30 new .rs files, ~10k lines).**

## 2. Pass Slicing Decisions (Creator Wave — file-level dependency graph, max parallelism)

**Workflow amendment (user-directed 2026-08-03, now permanent):** the test flow is xfstests-integration-only. There are **no Creator-synced per-pass Checker passes** anymore. The only runtime validation gate is the **meso-integration xfstests Checker** scheduled after implementation + Reviewer stabilize. Before the code is complete, the **Reviewer** is the only quality gate (static review; the user explicitly requests the pre-checker structural-audit role of PROTOCOL §1 rule 16). Creator passes are **command-free** and receive **no per-pass compile preflight** — compile evidence is owned by the integration Checker. `PROTOCOL.md` §1 rules 5 and 16 were amended permanently in `ad5abac3a`.

**Slicing rule:** each Creator pass owns a **disjoint write-set**. Passes that must edit shared files (crate-root `mod.rs`, `mount/superblock.rs`, `mount/build.rs`, `projection/inode.rs`, `projection/entry.rs`, `projection/binding_cache.rs`, `projection/mod.rs`) are serialized; all frozen cross-meso carrier extensions and consumption widenings are consolidated into one **seam-placement pass** so the four meso leaf passes never touch a shared file and run in parallel.

### 2.1 File-level dependency graph (why the waves look like this)

```text
Wave 1 (serial):  pass_01 mount_resource_policy          mount/*  (+ crate-root mod.rs: mod mount;)
                      │  consumed by every meso (OverlayFs, MountPolicy, OverlayLayerStack, claims,
                      │  UpperFilesystemCapabilities, CreatorCredentialPolicy); meso-02 additionally
                      │  EXTENDS OverlayFs (mount/superblock.rs, mount/build.rs) -> Wave 2 must follow
                      ▼
Wave 2 (serial):  pass_02 visibility_projection_identity  projection/*  (+ mount/superblock.rs,
                      │  mount/build.rs extensions; crate-root mod.rs: mod projection;)
                      │  consumed by meso 03-06 (OverlayInode, facts, RealObject, BindingCache,
                      │  InodeCache, IdentityPolicy, store_lower_id/read_lower_id); meso 03-06 also
                      │  extend/widen meso-01/02 shared carriers -> Wave 3 consolidates those edits
                      ▼
Wave 3 (serial):  pass_03 shared_carrier_seams            shared carrier fields + widenings +
                      │  crate-root module declarations + AccessType (NO feature claims; seam placement only)
                      ▼
Wave 4 (PARALLEL x4, disjoint write-sets):
  pass_04 merged_directory_index         readdir_index.rs            (4 micros)
  pass_05 copyup_authority_file_views    copyup/*  (5 files)          (17 micros)
  pass_06 metadata_security_xattr_policy metadata_security/* (4 files)(4 micros)
  pass_07 namespace_mutation_whiteout    dir/*  (6 files)             (11 micros)
                      │
Wave 5 (serial, per-meso):  Reviewer static gate (only pre-code-completion quality gate)
Wave 6 (serial):  meso-integration Checker (compile + full kernel build + overlay xfstests
                  suite per the six Designer validation contracts) — the single runtime gate
```

**Why Wave 1 and Wave 2 cannot run in parallel:** meso-02's Creator extends `OverlayFs` in `mount/superblock.rs` (fields `bindings`/`inodes`/`identity`) and `mount/build.rs` (`OverlayFs::new` extension: `AnonDeviceId` + `IdentityPolicy` construction) — the same files meso-01's Creator creates. Write-sets conflict, so Wave 2 waits for Wave 1.

**Why Wave 3 must be serial:** the four leaf mesos all need edits inside meso-01/02 files: `projection/inode.rs` (meso-03 `readdir_index` field, meso-04 `copyup_transition` field), `mount/superblock.rs` (meso-04 `workdir_temp_serial`, meso-05 `xattr_policy`, meso-06 `whiteout_cache`), `projection/entry.rs` (meso-03 + meso-06 `RealObject::new` / `is_opaque_directory`), `projection/binding_cache.rs` (meso-06 `BindingKey::new` + positive/hidden construction), `projection/mod.rs` (meso-06 `project_new_upper` + meso-04 `record_copyup_transition` hook call), and crate-root `mod.rs` (all four module declarations + meso-05 `AccessType`). Running any two of these leaf passes against the same shared file would corrupt the write-set boundary, so the frozen extensions/widenings are consolidated into one seams pass with a fully enumerated inventory (below).

### 2.2 The seven Creator passes

| Pass ID | Parent Meso | Covered micros (`需要实现`) | Write-set (exact) | Risk | Wave |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `pass_01_mount_resource_policy` | `mount_resource_policy` (01) | P0-01, P0-02, P0-03, P0-05, P0-18, P1-19, P1-20, P1-35, P2-11 (9) | `kernel/src/fs/fs_impls/overlayfs/mount/{mod.rs,options.rs,layers.rs,claims.rs,policy.rs,superblock.rs,build.rs}` (new); crate-root `overlayfs/mod.rs` (add `mod mount;`) | High (claims/IU, construction) | 1 |
| `pass_02_visibility_projection_identity` | `visibility_projection_identity` (02) | P0-04, P0-06, P0-07, P0-08, P0-09, P0-10, P0-11, P0-12, P0-16, P0-17, P1-07, P2-01 (12) | `overlayfs/projection/{mod.rs,inode.rs,entry.rs,binding_cache.rs,inode_cache.rs,lower_id.rs,identity.rs}` (new); `mount/superblock.rs` + `mount/build.rs` (meso-02 field additions + `OverlayFs::new` extension); crate-root `overlayfs/mod.rs` (add `mod projection;`) | High (DIR/INODE domains, caches) | 2 |
| `pass_03_shared_carrier_seams` | N/A — cross-meso shared carriers (foundation; **no feature claims**) | N/A (seam placement for the micros listed in §2.3 only) | `mount/superblock.rs`, `mount/build.rs`, `projection/inode.rs`, `projection/entry.rs`, `projection/binding_cache.rs`, `projection/mod.rs`, `projection/identity.rs` (only if `project_object_id` needs widening — currently already `pub(super)`), crate-root `overlayfs/mod.rs` (declare `mod readdir_index; mod copyup; mod metadata_security; mod dir;` + `AccessType` enum) | Normal | 3 |
| `pass_04_merged_directory_index` | `merged_directory_index` (03) | P0-13, P0-14, P0-15, P1-31 (4) | `overlayfs/readdir_index.rs` (new) | High (cookie/tombstone) | 4 |
| `pass_05_copyup_authority_file_views` | `copyup_authority_file_views` (04) | P1-01, P1-02, P1-03, P1-04, P1-05, P1-06, P1-08, P1-09, P1-10, P1-11, P1-12, P1-13, P1-14, P1-15, P1-32, P1-34, P1-37 (17) | `overlayfs/copyup/{mod.rs,coordination.rs,trigger.rs,promote.rs,workdir.rs}` (new) | High (CUL, promotion) | 4 |
| `pass_06_metadata_security_xattr_policy` | `metadata_security_xattr_policy` (05) | P1-16, P1-17, P1-18, P1-33 (4) | `overlayfs/metadata_security/{mod.rs,permission.rs,metadata.rs,xattr.rs}` (new) | High (permission surface) | 4 |
| `pass_07_namespace_mutation_whiteout` | `namespace_mutation_whiteout` (06) | P1-21, P1-22, P1-23, P1-24, P1-25, P1-26, P1-27, P1-28, P1-29, P1-30, P1-36 (11) | `overlayfs/dir/{mod.rs,create.rs,remove.rs,link.rs,rename.rs,whiteout.rs}` (new) | High (two-parent DIR, WL) | 4 |

Covered-micro union: 9+12+4+17+4+11 = **57** — exactly the Stage-D `需要实现` set, no `暂不实现` ID is touched.

### 2.3 `pass_03_shared_carrier_seams` — frozen inventory (all taken verbatim from the accepted specs)

The seams pass implements **no micro-feature**; it places the already-frozen carrier surfaces so the four leaf passes have disjoint write-sets. Every item below is a recorded requirement in the accepted meso-03/04/05/06 specs; the owning meso's Creator implements the actual feature logic in its own files. If a widening is refused at implementation time, the packet escalates to the main agent instead of inventing a parallel interface.

1. `projection/inode.rs`: add `readdir_index: Option<Mutex<ReaddirIndex>>` (meso-03 spec §4) and `copyup_transition: Mutex<Option<CopyUpTransition>>` (meso-04 spec §4.1) to `OverlayInode`, with initialization in the meso-02 constructors `new_root` / `project_inode` (meso-03 spec §4: `Some` iff directory for the index; meso-04: `None` initially). Widen `facts_snapshot()` and `dir()` from private to `pub(super)` (meso-03 §4.1 / meso-04 §3.4 item 1). Make `OverlayObjectFacts::{kind, upper, lowers}` readable at `pub(super)` (meso-03 §4.1).
2. `projection/entry.rs`: add `pub(super) RealObject::new(layer_index, real_inode, fsid, container_dev_id)` and widen `is_opaque_directory()` to `pub(super)` (meso-03 §4.1 / meso-06 §4.1).
3. `projection/binding_cache.rs`: add `pub(super) BindingKey::new(parent_id: RealObjectKey, name: String)` and `pub(super)` construction for `PositiveBinding`, `NegativeBinding::HiddenByWhiteout`, and `HiddenEvidence` (meso-06 §4.1).
4. `projection/mod.rs`: add the `pub(super) OverlayFs::project_new_upper(&self, facts: &OverlayObjectFacts) -> Arc<OverlayInode>` seam (meso-06 §4.1; reuses `project_inode` semantics) and the `record_copyup_transition(publication_parent: Arc<OverlayInode>, name: &str)` invocation at the positive-binding assembly point (meso-04 §3.4 item 2 / §4.1 hook — once per inode, first positive binding wins; the method body is implemented by pass_05 in `copyup/mod.rs`).
5. `mount/superblock.rs`: add `workdir_temp_serial: AtomicU64` (meso-04 P1-34), `xattr_policy: OverlayXattrPolicy` (meso-05 P1-33), `whiteout_cache: Mutex<WhiteoutCache>` (meso-06 P1-36). `mount/build.rs`: initialize them in `OverlayFs::new` (forward references to `ReaddirIndex` / `CopyUpTransition` / `OverlayXattrPolicy` / `WhiteoutCache` types defined by the leaf passes are expected; the tree compiles only after Wave 4 — accepted, since no per-pass compile gate exists).
6. crate-root `overlayfs/mod.rs`: declare `mod readdir_index; mod copyup; mod metadata_security; mod dir;` and declare the shared `AccessType { ReadOnly, Mutating }` enum at `pub(in crate::fs::fs_impls::overlayfs)` (meso-05 revision-01 promotion).
7. `projection/identity.rs`: verify `IdentityPolicy::project_object_id` is `pub(super)` (already is per meso-02 spec §4); no change expected.
8. **Recorded non-edits:** meso-01 capability accessors (`can_mknod_char`, `can_store_private_xattr`), `policy()`/`claims()`, `MountPolicy::is_effective_read_only`/`is_default_permissions` are already published `pub(super)` by meso-01 revisions 04/05 — no widening needed. `RealObject::{layer_index, real_inode, fsid, container_dev_id}`, `OverlayInode::{key, object_id}`, `BindingCache::{get, insert, invalidate}`, `store_lower_id`/`read_lower_id`, `ensure_upper_authority`/`select_real_inode`/`fs_arc`/workdir-temp methods, `check_permission(AccessType, Permission)`, `OverlayXattrPolicy::{classify, is_private}` and `xattr_policy()` are all already `pub(super)` per the frozen specs — no seams pass work.

**Meso-03-owned seams (implemented by pass_04, NOT the seams pass):** `invalidate_readdir_index()`, `readdir_index_insert(name, inode, type_)`, `readdir_index_remove(name)`, `visible_child_count(&self, facts)` (meso-06 §4.1 / meso-03 §4) — these compose the `ReaddirIndex` payload and belong to meso-03's file.

### 2.4 Reviewer + integration gates (workflow amendment consequences)

> **Superseded note (2026-08-03):** the per-meso "Wave 5 Reviewer" sketch below was
> replaced by the user-confirmed §2A orchestration — a **diff-mode review after each
> Wave** (3-persona fan-out) with long-lived repair loops. It is kept for the record.

- **Reviewer:** runs per Wave (not per Meso) via the aster-code-review diff pipeline (§2A.2); it is the **first and only static gate** before code completion; direct edits limited to line-level non-functional fixes; structural findings route back to the owning repair Creator.
- **Integration Checker:** one meso-integration Checker pass (or a small bounded set) per the six Designer validation contracts: repository-entry compile + full kernel build + the overlay xfstests suite (`overlay/001`-`078`, `100`, `101` per `full.list`), reporting mapped/observed/not-run per contract. This is the **only** runtime validation. Deferred rows from the meso-01..06 validation contracts (e.g., the meso-01 mount group's runtime deferral, meso-02/03 readdir-merged pickup, meso-04/05/06 integration obligations) are resolved here.


## 2A. Agreed Subagent Orchestration (user-confirmed 2026-08-03)

This section is the authoritative dispatch/review/commit contract for the Creator wave. It supersedes the "Wave 5 per-meso Reviewer" sketch in §2.4/§5 with the user-confirmed plan below.

### 2A.1 Dispatch granularity — one Creator per file per Wave

- Every `.rs` file a Wave produces is one Creator task (disjoint write-sets by construction).
- Task ids follow `task_creator_w<N>_<component>_<file>`; dispatch packets live under
  `.agents/subagent-tasks/<component-id>/` and Creator receipts under
  `.agents/components/<component-id>/` (both gitignored — no git pollution).
- Each packet names the exact file, the file-scoped micro slice (from the Designer spec's
  module-layout annotations), and the legacy-reference boundary (§4 decision 6).
- Concurrency: batch by budget (default ≤8 alive at once); close each Creator immediately
  after its receipt is accepted.

### 2A.2 Wave-end Review — aster-code-review in **diff** mode (user ruling)

- **Mode: `diff <base> <output>`**, NOT files mode — the reviewed range
  `merge-base(<base>, HEAD)..HEAD` covers only the current Wave's commit, so
  already-approved code from earlier Waves is never re-reviewed.
- **Base for Wave N = the accepted commit of Wave N-1** (Wave 1 base = the baseline
  commit `B` created 2026-08-03). A Wave must therefore end in a commit before review.
- **Personas (path-activated, fan-out = 3 subagents):** maintainability + development
  (Correctness) + security. hardware/documentation are not activated for overlayfs `.rs`.
  One `spawn_agent` per persona, initial message = the exact
  `build_pass_prompt.sh <input> <persona>` output (deterministic scripts run by the main
  agent; `codex exec` is NOT run — direct subagents per user ruling).
- Pipeline: `resolve_target.sh '<raw args>'` → `build_pass_prompt.sh` per persona →
  spawn/fan-out → collect per-persona JSON → `assemble_review.sh` → main agent verifies
  (refute/uncertain/confirmed) and consolidates → writes the summary.
- **Artifacts:** the review packet (raw arg string, meta, persona list, scope rules) goes
  under `.agents/subagent-tasks/<wave-review-group>/`; the assembled review file goes under
  `.agents/components/<wave-review-group>/` — both gitignored, never committed.
- **Scope:** reviewer reviews only the Wave's `.rs` delta; all `.agents/` markdown records
  are out of scope (diff range is code-only by construction — see 2A.4).

### 2A.3 Repair loop — long-lived Reviewer + Creator (user ruling)

- On findings: spawn ONE repair Creator (long-lived) per Wave; route repair batches via
  `send_input`. Do NOT re-spawn reviewers or creators per round — reuse the same agents.
- After each repair round: `git add` + `git commit --amend` the Wave's commit, then
  re-review with the SAME base (`diff <base>` — only the amended Wave delta is re-read).
- Loop until the review is clean; then close the repair Creator and the persona reviewers.
- During repair, at most 4 agents are alive (3 reviewers + 1 repair Creator); they do not
  overlap Creator fan-out batches.

### 2A.4 Commit discipline (main agent owns amend-vs-split)

- Baseline commit `B` (2026-08-03): legacy `fs.rs` → `legacy_fs.rs` rename + `mod.rs` +
  the current durable records (PASS_SLICING, SYSTEM_BLUEPRINT, handoffs). Normal commit.
- Each Wave ends in its own commit (`wip(overlayfs): wave N <component> …`); repair rounds
  `--amend` that same commit. Wave acceptance stabilizes it; the next Wave's base = it.
- **Durable records (tracked `.agents/*.md`) are NOT committed during the Wave cycle** —
  they stay as working-tree edits so every Wave commit / review diff is `.rs`-only. They are
  committed at integration/acceptance (or on explicit user request), never inside a reviewed
  commit series.
- Split/cancel-WIP trigger (my decision, per user delegation): a Wave is cancelled/rewritten
  only via `git reset` of its WIP commit; a new commit replaces the WIP only when the Wave's
  contract materially changes mid-cycle.

## 3. Thread Activity Log (2026-08-03 session)

- Read the full intake (README, PROTOCOL, SYSTEM_BLUEPRINT, PASS_SLICING, latest handoff, all six accepted Designer specs + validation contracts, meso-07 disposition, Creator/Reviewer protocols, dispatch + report templates).
- Verified the file-level dependency graph and the exact shared-file write conflicts from the accepted specs' consumption-seam tables (meso-03 §4.1/§5.3, meso-04 §4.1, meso-05 §7, meso-06 §4.1) and the meso-02/04 `record_copyup_transition` hook contract.
- Marked `20260801-state-cleanup-designer-prep_main_agent_handoff.md` ENDED/SUPERSEDED with a banner and updated status.
- Created this live handoff and recorded the pass slicing in `PASS_SLICING.md` (new `creator_pass_slicing_20260803` block) and `SYSTEM_BLUEPRINT.md` (Phase 3 closed; Phase 4 opened; pipeline index + notes updated).
- **Legacy file rename (user-directed 2026-08-03):** `git mv` `fs.rs` →
  `legacy_fs.rs` (content unchanged except a header banner marking it LEGACY /
  FROZEN / NOT A DESIGN SOURCE); crate-root `mod.rs` updated to
  `mod legacy_fs; use legacy_fs::OverlayFsType;`. The legacy `OverlayFsType`
  registration in `overlayfs::init()` is unchanged and remains the ACTIVE
  registered overlay filesystem until an explicit takeover decision.
- **Orchestration agreed (2026-08-03):** per-file Creators per Wave; Wave-end
  review in `diff` mode (base = previous accepted Wave commit) via the
  aster-code-review pipeline adapted to direct subagents (3 persona fan-out);
  long-lived repair Reviewer+Creator; per-Wave commits amended per repair
  round; review artifacts under gitignored `subagent-tasks/` + `components/`;
  durable records uncommitted during the Wave cycle. Recorded as §2A.
- **Wave 1 EXECUTED and ACCEPTED (2026-08-03):** 7 per-file Creators produced the `mount/` module tree; wave commit amended through 2 repair rounds (`e4b5c0b27` → `dafd6a38e` → `e1613f12c`); aster-code-review diff-mode review (3 personas, base `be0e574c5`) ran 3 rounds — development 5→2→0, security 3→1 (recorded limitation), maintainability 15→8→5 (all accepted-with-note). Review artifacts under `.agents/components/wave_01_review/` (rounds 1-3). `e1613f12c` is the stable Wave-1 commit and the Wave-2 review base.
- **Wave 2 EXECUTED and ACCEPTED (2026-08-03):** 9 per-file Creators landed `projection/*` + the superblock/build extensions; commit amended through 3 repair rounds (`157be7812` → `2787a56aa` → `23454f509` → `77b0d4a49`); diff-mode review (base `e1613f12c`) ran 4 rounds — development 8→4→2→1, security 2→1→1→0, maintainability 8→3→2→3 (all remaining accepted-with-note: frozen `is_directory` signature, P1-07 durable-origin record format limitation, claims() widening, no-unit-tests/ktest-forbidden). Review artifacts under `.agents/components/wave_02_review/`. `77b0d4a49` is the stable Wave-2 commit and the Wave-3 review base.
- **Wave 3 EXECUTED and ACCEPTED (2026-08-03):** 7 seam-placement Creators landed the frozen cross-meso carrier extensions/widenings; commit amended through 6 repair rounds (`9f64fe744` → … → `b9b9d6caf`); diff-mode review (base `77b0d4a49`) ran 6 rounds — development 3→2→1→1→1→0, security 2→1→1→1→1→0, maintainability 6→4→2→2→1→2(nits). Review artifacts under `.agents/components/wave_03_review/`. `b9b9d6caf` is the stable Wave-3 commit and the Wave-4 review base.
- **Wave 4 EXECUTED and ACCEPTED (2026-08-03):** 16 per-file Creators (+ 3 reconciliation/assembly passes) landed the four leaf mesos (`readdir_index.rs`, `copyup/*`, `metadata_security/*`, `dir/*`); commit amended through 7 repair rounds (`ad079260f` → `43a0747bc`); diff-mode review (base `b9b9d6caf`) ran 8 rounds — development 2→0→0→1→1→1→1→0, security 4→1→1→0→0→0→1→0, maintainability 15→4→2→2→2→0→0→0. The metadata-setter chain (rounds 4-8) closed the chmod/chown/utimensat security and correctness findings with Linux-faithful gates. Review artifacts under `.agents/components/wave_04_review/`. `43a0747bc` is the stable Wave-4 commit.
- **ALL FOUR IMPLEMENTATION WAVES COMPLETE AND ACCEPTED (2026-08-03).** The 57-micro `需要实现` set is fully implemented across `mount/` + `projection/` + `readdir_index.rs` + `copyup/` + `metadata_security/` + `dir/`; the legacy registration remains active (takeover deferred). The bounded pre-wave5 revision is now accepted at `bb387a8ef`: it includes xino plumbing, origin wire v3, the complete C2 retry, and all six mechanical repairs. The static Wave5 Checker-owned compile/lint lane is the current action; the meso-integration xfstests Checker remains wave7+.

## 4. Explicit Agent-Level Decisions

1. **Workflow amendment (user-directed, permanent):** Creator-synced per-pass Checker passes are eliminated for this refactor; Reviewer is the only pre-code-completion gate; the meso-integration xfstests Checker is the single runtime gate; Creator passes are command-free with no per-pass compile preflight. Recorded in `PASS_SLICING.md`; `PROTOCOL.md` §1 rules 5 and 16 were permanently amended in `ad5abac3a`.
2. **Seams pass:** the frozen cross-meso carrier extensions / consumption widenings are consolidated into `pass_03_shared_carrier_seams` (parent N/A, no feature claims) so the four leaf passes are write-disjoint and run in parallel. This is a deliberate scheduling arrangement; it changes no Designer contract and claims no micro.
3. **Module wiring:** crate-root `mod.rs` edits are confined to the serial waves — `mod mount;` in pass_01, `mod projection;` in pass_02, and all four leaf declarations + `AccessType` in pass_03 — so no leaf pass edits the crate root.
4. **Compile posture:** no Creator pass receives compile capability; intermediate states between waves are expected not to compile (forward references to leaf types from the seams pass). The integration Checker owns the first compile.
5. **Takeover:** the legacy `legacy_fs.rs` remains the registered filesystem; registering `mount::OverlayFsType` in `overlayfs::init()` is a takeover decision deferred to after the integration Checker (per the meso-01 spec, `init()` is "unchanged" this wave).
6. **Legacy-reference boundary (user-directed 2026-08-03):** the old single-file
   implementation was renamed `fs.rs` → `legacy_fs.rs` so it cannot collide with
   the refactor module tree and stays available for ONE purpose. Every Creator
   (and Reviewer) packet MUST state: the **only** permitted reference to
   `legacy_fs.rs` is the **registration wiring** — the `OverlayFsType` `FsType`
   impl and the `register()` invocation shape in `overlayfs::mod.rs::init()`.
   All other content (layout, structures, lock handling, recipes, option
   parsing) is **forbidden** as a reference; Creators must implement from the
   Designer specs, the design documents (`designdoc/`), and the staged priors
   (`priors/`) only. Spec references to "the legacy `fs.rs`" now denote
   `legacy_fs.rs` (path rename only, no content change).

## 5. Next Actions for the Next Thread (CRITICAL)

**Phase 4 (four implementation Waves + Review/Repair) is COMPLETE and ACCEPTED.**

**Recorded scheduling decision (user-ruled 2026-08-03) — the post-implementation gate structure:**

| Phase | Scope | Owner |
| :--- | :--- | :--- |
| **pre-wave5** | Resolve **compile-unrelated omissions** — known gaps, design decisions, traceability/ledger audits, and bounded behavioral-fidelity fixes that do not need the compiler to locate (they are already recorded in this handoff and the wave reviews). No compile/lint run required to scope them. | Main agent + user decisions + bounded Creator cleanups |
| **wave5** | **Compile + lint repair loop** — first `cargo check`/`make kernel` build, then a Checker-routed compile/lint repair loop (stale `#[expect(dead_code)]`, `unfulfilled_lint_expectations`, unused-import/private-interfaces warnings, trait/borrow errors) until the tree builds clean under `make check`. | Checker + bounded repair Creators |
| **wave6** | **Comment documentation** — a dedicated doc pass: align module/struct/method docs with the landed behavior, backtick identifiers, third-person present, remove any stale round-by-round narratives left by the repair loops. | Bounded doc Creator(s) |
| **wave7+** | **xfstests debugging** — the meso-integration xfstests Checker runs the overlay suite per the six Designer validation contracts; iterative debugging of mapped/observed/not-run rows until the intended upstream suite behavior is evidenced. | Checker (`$ovfs-checker` lane) |

**Immediate next thread step:** run Checker continuation 01 from
`subagent-tasks/wave_05_compile_lint/` using only the exact protocol
target-specific cargo-check command. Apply only clearly mechanical
name/import/visibility/interface-propagation repairs; escalate VFS ownership,
lifecycle, locking, semantics, or non-obvious borrows to the user. Parent
identity and executable credential scope remain deferred limitations and must
not be silently reintroduced.

**Closure Designer dispatch (2026-08-04):** dispatched
`task_designer_pre_wave5_closure_20260804` through
`subagent-tasks/pre_wave5_closure/pre_wave5_closure_designer_dispatch.md`.
The Designer may create only the paired artifacts under
`components/pre_wave5_closure/`; it must refine the already adjudicated C2
six-file typed retry, P3 origin triplet, and six mechanical repairs without
reopening P1/P2, lock topology, or pass slicing. Artifacts are pending
structural acceptance before any Creator packet is scheduled.

**Closure Designer acceptance (2026-08-04):** accepted structurally. The
paired `pre_wave5_closure_designer_spec.md` and
`pre_wave5_closure_designer_validation.md` fully define the C2 six-file typed
retry, P3 v3 triplet and conservative resolution, and six mechanical repairs;
they preserve the topology, record P1/P2 as deferred gaps, and retain the
xfstests-only boundary. Creator pass slicing is the next main-agent decision;
no production implementation is accepted or dispatched by this design task.

**Closure Creator slicing (2026-08-04):** the accepted closure design is split
into three serial Creator phases under `pre_wave5_closure_creator_slicing_20260804`:
C2 workdir retry (six files, `P1-34`), P3 origin triplet (four files after the
required `readdir_index.rs` record-consumer propagation,
`P1-07`, including the inseparable `IdentityPolicy::layer_devs` removal), then
the five remaining mechanical repairs. One long-lived Creator executes all
three phases. The main agent reviews each exact diff and routes any repair to
that same Creator; accepted Rust changes amend the bounded pre-wave5 commit
phase by phase.
This replaces neither the permanent Checker gate nor adds a Reviewer wave.

**Closure execution and acceptance (2026-08-04):** Phase A accepted the full
typed C2 retry; Phase B accepted v3 origin triplet persistence, the immutable
lower snapshot, unique pair resolution, its required `readdir_index.rs`
consumer propagation, and `IdentityPolicy::layer_devs` removal; Phase C
accepted the five remaining mechanical repairs. The main agent returned two
P3 presentation corrections to the same Creator and then amended each phase's
exact Rust write-set. Final commit `bb387a8ef` has no `WIP`; no Reviewer was
introduced. The pre-wave5 closure is accepted and Wave5's Checker static lane
is open through `task_checker_wave5_compile_lint_20260804`.

### Pre-wave5 decision history (user-ruled 2026-08-03; status reconciled 2026-08-04)

**A1 — `xino=`: DONE in `ad5abac3a`.** The original omission was a frozen-contract gap, not a missing projection matrix. The bounded meso-01/02 revision added `MountOptionKey::Xino`, parses `xino=off|auto|on`, stores the mode in `MountPolicy`, passes the eighth `assemble` parameter, moved `XinoMode` to `mount/options.rs`, and made `IdentityPolicy` consume the selected mode. This item is historical context only and MUST NOT be presented as pending.

**A2 — P1-07 durable-origin record format (explained in detail; user-directed fix 2026-08-03):** `LowerIdRecord` is the durable `trusted.overlay.origin` xattr (written by meso-04 copy-up, read by meso-02 identity) intended to keep `st_ino` stable across copy-up. The original v1 native wire was **24 bytes** (8-byte header + 16-byte payload `fsid: u64` + `real_ino: u64`); the current v2 wire remains 24 bytes but replaces `fsid` with `container_dev_id`.

**What `fsid` actually is (code-verified):** within a mount it is a **per-unique-underlying-filesystem-instance ordinal** — `OverlayLayerStack::assemble` dedups by `Arc::ptr_eq` on the underlying `Arc<dyn FileSystem>` and assigns ordinals in stack order (upper = 0, then lowers in option order; Linux `ovl_layer[].fsid` layout). So it is "an identifier of the underlying fs" only in the dedup sense: the same underlying fs instance always gets the same ordinal **within one mount**, but the **value is an order-dependent index re-assigned from scratch at every mount** — it is NOT a stable property of the filesystem.

**Why the conflict arises (the user's question):** the record is **durable** (persisted on the upper) while the ordinal is **mount-local**. The mapping ordinal→fs is rebuilt by enumeration order at every mount. If a later mount changes the lowerdir order, adds/removes a layer, or mounts the same upper with a different stack, the persisted `fsid` value now names a **different** underlying fs (or none). The wave-2 membership check (`fsid ∈ current lower ordinals`) catches ordinals that no longer exist, but **cannot catch ordinal collisions** — a stale `fsid=1` under a mount where ordinal 1 is a different fs passes the check and projects `real_ino` against the wrong layer → fabricated/colliding `(st_dev, st_ino)`. Linux has the identical mount-local `fsid` — which is why Linux **never persists it**: the Linux origin xattr stores a real file handle carrying the underlying fs's stable identity (uuid-based), not a layer index. Asterinas has no export-FH surface (recorded absence, meso-02 §3.5), so the native record used the ordinal pair — exactly where the durability gap entered.

**Directed fix status: DONE in `ad5abac3a`.** Wire v2 persists `(container_dev_id, real_ino)`; v1 ordinal records decode conservatively to `Ok(None)`; a current per-mount lower ordinal is derived only after matching the stored device against the current lower set. This closes the old mount-local ordinal-reuse collision. It does **not** disambiguate two current lower filesystem instances that expose the same `st_dev`; §5A.3 separately records the accepted triplet follow-up and MUST NOT be confused with the completed A2 migration.

**A3 — WL lock type: CLOSED (user-ruled).** `OverlayFs::whiteout_cache: Mutex<WhiteoutCache>` stays **`Mutex`**. The bounded single-slot cache (pop/push/disable-sharing, no BIO under the guard) does not warrant an `RwMutex`; the deferred `Mutex`-vs-`RwMutex` ledger item is now closed (post-integration re-evaluation only if profiling shows contention).

**A4 — `nlink != 1` handling: NON-ITEM, corrected framing (user-ruled 2026-08-03).** "Not maintaining nlink" does **not** mean skipping `nlink != 1` targets — it means the design does **not maintain an index / reverse mapping** for hardlinked lower targets (that is the deferred `P2-07`/index feature). The accepted consequence: a **second copy-up of the same lower hardlink may produce a distinct upper file** (the hardlink relationship is not preserved in the upper) — Linux considers this acceptable when the corresponding option (index) is off. On the `st_ino` side, this wave keeps the **constant-origin-ino invariant** (consume the origin record whenever present, not applying the `nlink == 1` gate) — a documented divergence from Linux's `nlink > 1`-uses-upper-ino behavior, with the broken-lower-hardlink collision corner recorded and gated by `P2-07`/`P3-01`. This is a **confirmed design decision**, not a gap; removed from pre-wave5 (ledger item only).

**A5 — legacy takeover: SCHEDULED INTO WAVE5 (user-ruled).** During wave5, once the tree compiles clean: switch `overlayfs::init()` to register **`mount::OverlayFsType`** (the new implementation becomes the ACTIVE registered overlay filesystem), remove the `#[expect(dead_code)]` on `OverlayFsType`, and delete the now-unregistered `mod legacy_fs;` wiring — `legacy_fs.rs` stays as a frozen archive file (its top-level `#![expect(dead_code)]` keeps it lint-clean while unregistered); the physical deletion of `legacy_fs.rs` is a final-acceptance decision.

**A6 — PROTOCOL §1 rule 5 amendment: DONE (2026-08-03).** `PROTOCOL.md` rule 5 rewritten to make the no-per-pass-Checker workflow **permanent** (wave-level Reviewer gate + single meso-integration xfstests Checker; no per-pass compile preflight), and rule 16 annotated that the pre-code-completion Reviewer gate is the default for this refactor.

**Remaining pre-wave5 work after status reconciliation:**

1. **Decided P3 implementation:** upgrade the native origin record to
   `(container_dev_id, lower_layer_root_ino, real_ino)` and resolve its current
   mount-local `fsid` only from a unique `(device, lower-root-ino)` match
   (§5A.3 P3). Preserve the conservative `None` fallback for simultaneous
   ambiguity. This is an accepted native approximation, not Linux-equivalent
   UUID/FH identity.
2. **Required C2 implementation:** after a bounded Designer addendum, replace
   the private copy-up-only retry helper with the shared, explicitly typed
   workdir-temp request/retry contract in §5A.4. It MUST cover
   `copyup/promote.rs`, `copyup/workdir.rs`, `dir/remove.rs`, `dir/create.rs`,
   `dir/link.rs`, and `dir/whiteout.rs`; use one fresh name per retry, one
   shared bound, and retry only on `EEXIST`. A partial repair is not
   acceptable.
3. **Mechanical work:** repair all six items in §5A.5 (user decision
   2026-08-04). The readdir documentation repair must use the §5A.4 C1 wording;
   it must not claim that valid cached entries and returned facts share one
   build epoch.
4. **B audits (read-only):** produce the consolidated 57-micro -> file ->
   symbol ledger; verify the 24 deferred micros remain untouched; build the
   deviation register from all Creator reports and Wave reviews, classified as
   accepted / needs-fix / needs-decision.
5. **D environment preparation:** preserve the xfstests harness/config
   obligations from `XFSTESTS_PREBUILT_IMAGE_GUIDE.md` and the pass-00 baseline
   for wave7+; archive the VFS-side dependencies
   (task_ctx/OverlayInuseSlot/mmap write-intent/fadvise/splice) in an explicit
   external-dependency register. This is wave7 preparation, not a substitute
   for the current pre-wave5 work.

**Explicit non-blockers / no-dispatch boundary:** overlay-parent identity (P1)
and executable creator credentials (P2) are deferred by user ruling. C1
requires only the mechanical documentation correction now; its remaining
semantic exposure is P1. C2 is a required pre-wave5 repair under the exact
six-file boundary below, and it blocks wave5 until accepted. No P1/P2 work may
be added to a repair packet or used to block wave5 without a later user ruling.

## 5A. Authoritative Pre-Wave5 Manual-Adjudication Register (2026-08-04)

This section supersedes the 2026-08-03 session-close classification. Parent
identity and executable creator credentials are explicitly **DEFERRED known
gaps**, not accepted semantics and not current wave5 blockers. Complete
workdir-temp retry (C2) is promoted by user direction to a **REQUIRED
pre-wave5 correctness repair** under its full six-file scope. Same-`st_dev`
origin disambiguation is decided as the bounded native triplet upgrade in P3;
its stronger Linux UUID/FH gap remains recorded but does not block wave5. The
round-9 batch also is not one cosmetic/lint class: it mixed broad
semantic/correctness work with six bounded mechanical cleanups.

The review/repair cycle ran nine rounds and repeatedly re-emitted the same
issues. PROTOCOL §1 rule 10 requires escalation after five failed loops. This
register records the resulting manual gate; the next action is the bounded
sequence in §5A.7, not an ordinary round-10 review/repair continuation.

### 5A.1 Classification Rule

- **Linux-reference / manual design decision:** the defect crosses a semantic
  boundary (VFS parent context, process credentials, or durable filesystem
  identity). A local edit chosen without reference behavior could establish
  the wrong contract.
- **Correctness impact:** the intended local invariant is clear enough that
  leaving the gap can change operation success or make a published consistency
  claim false. C2 is a required pre-wave5 repair and blocks wave5 until its
  complete scoped implementation is accepted.
- **Mechanical repair:** behavior and ownership are already decided; the edit
  removes misleading text, unused receiver/state, or a surprising expression.
  The user ruled on 2026-08-04 that all such items MUST be repaired.
- **Deferred known gap:** the defect is real, but the required VFS surface or
  cross-module repair is too broad for the current pre-wave5 scope. Deferral
  does not accept the behavior; it prohibits dispatch until a later user
  ruling and does not block wave5.
- **Closed/history:** already implemented or explicitly accepted; do not
  re-open merely because a fresh reviewer lacks the disposition record.

Classification follows consequence and missing design authority, not the
persona or severity label that happened to report the item.

### 5A.2 Closed / Historical Items

1. **A1 `xino=`:** implemented in `ad5abac3a`; see §5. It is not pending.
2. **A2 mount-local ordinal collision:** origin wire v2 now persists
   `(container_dev_id, real_ino)` and rejects v1 conservatively. This fixes
   remount/layer-order ordinal reuse, but not two current lowers sharing one
   device ID.
3. **A3 whiteout-cache lock:** `Mutex` retained by user decision.
4. **A4 no-index hardlinks:** accepted no-index behavior; not a pre-wave5 gap.
5. **A6 workflow amendment and C4 supplementary groups:** complete.
6. **Xattr failure policy:** the local split itself is decided: persistent
   copy-up is `Strict`; clear-empty removal is `BestEffort`. The remaining
   creator-credential execution gap is separately open below.

### 5A.3 Linux-Reference / Manual Design Decisions

#### P1. Overlay-parent identity for `d_ino("..")`

**Disposition (user ruling 2026-08-04): DEFERRED / KNOWN LINUX GAP.** The
repair is too broad for the current wave. Record the reference behavior and
the local mismatch, but do not add a VFS interface, an inode-extension
backlink, or another parent carrier in this pre-wave5 work. This item does not
block wave5 and must not be dispatched without a later user ruling.

**Principle.** `..` names the parent in the overlay namespace. Its inode
number should therefore be derived from the actual overlay parent carrier and
agree with that parent's `stat` identity, regardless of which real layer
supplies the child.

**Current defect.** `readdir_index.rs::resolve_parent_object_id` selects the
child's visible real source and calls `lookup("..")` on that source. This is
exact only when that real parent is also the visible carrier selected for the
overlay parent. For a lower-visible child beneath an upper/merged overlay
parent, it can project a lower real parent while `stat("..")` resolves the
higher overlay parent. The current self-parent fallback avoids unstable
allocation in some branches but does not establish the parent identity
contract.

**Linux behavior and Asterinas gap.** Linux
`fs/overlayfs/readdir.c:793-913` starts from the opened overlay
`file->f_path.dentry`, takes `dir->d_parent`, and obtains that overlay parent's
inode through `vfs_getattr`; it never reconstructs the parent by walking
`lookup("..")` from the child's visible real source. In Asterinas, name and
parent are Dentry state rather than intrinsic Inode state, and the current
`FileOps::readdir_at` receiver has no route back to the opened overlay Dentry.
`readdir_index.rs::resolve_parent_object_id` therefore uses the real-source
walk and has the mismatch described above. Fixing it requires carrying or
recovering overlay-Dentry context (or an equivalent VFS extension) across a
broader surface; there is no inode method that can recover its overlay name
and parent today. That scope is why the issue is deferred rather than locally
patched.

#### P2. Saved creator credentials versus executable credential scope

**Disposition (user ruling 2026-08-04): DEFERRED / PENDING DECISION.** The
stored snapshot and no-op execution seam remain a known limitation. A scoped
credential-override repair is considered too broad for the current bounded
work, so this item is not repaired now and does not block wave5. Do not turn
the deferral into acceptance of the current behavior, and do not dispatch a
credential/VFS repair without a later user decision.

**Principle.** Overlayfs performs underlying filesystem operations under one
stable credential policy, normally the mount creator's credentials, so the
same overlay operation does not acquire caller-dependent backing-fs behavior.

**Current defect.** `CreatorCredentialPolicy` does save a
`Credentials<ReadDupOp>` snapshot. However,
`with_creator_credentials_fn` currently calls `operation_fn()` directly. The
underlying `Inode::check_permission`, xattr reads/writes, and related VFS paths
implicitly consult `Task::current()`. Saving a value does not install it into
that implicit lookup, so an origin-xattr read and xattr copy still execute as
the current caller. This can make `d_ino("..")` reader-dependent and can make
an otherwise valid non-owner copy-up fail (strict policy) or lose metadata on
clear-empty (best-effort policy).

**Why this class.** Linux stores `creator_cred` and wraps overlay readdir and
backing operations in `override_creds`/`revert_creds` (`with_ovl_creds`). The
Asterinas gap is a scoped process/VFS credential-install mechanism with
well-defined restoration and nesting, not missing credential storage inside
overlayfs. The credential-dependent origin-xattr and backing-operation
symptoms share this root cause; they should not be repaired independently with
expanding error exceptions. This is separate from P1's primary
missing-Dentry-context cause, although it can add reader-dependent behavior to
P1's current fallback route.

#### P3. Multiple current lower filesystems sharing `st_dev`

**Disposition (user ruling 2026-08-04): IMPLEMENT THE NATIVE TRIPLET; ACCEPT
THE RESIDUAL COLLISION RISK.** Upgrade the origin wire to
`(container_dev_id, lower_layer_root_ino, real_ino)`. At read time, derive the
current mount-local `fsid` only when `(container_dev_id,
lower_layer_root_ino)` selects one distinct current lower filesystem; retain
the conservative `None` fallback when multiple distinct `fsid`s match. This
bounded native approximation is required before wave5. The complete-triplet
cross-remount collision described below is recorded as a known limitation and
is not repaired in this wave.

**Principle.** A persisted origin record must identify the original
filesystem/inode unambiguously across remounts. Device identity is sufficient
only when it is unique among the candidate lowers.

**Current defect.** Wire v2 stores `(container_dev_id, real_ino)`. When exactly
one distinct current lower `fsid` maps to that device, `IdentityPolicy` derives
its current mount ordinal and preserves projected identity (several layer roots
on the same underlying filesystem may share that one `fsid`). When two
distinct lower filesystem instances/`fsid`s share the same `st_dev`,
`resolve_layer_id_for_record` returns `None`; projection falls back to
visible-source identity. This is fail-safe against wrong-layer attribution,
but `st_ino` can change across copy-up and the promised durable continuity is
lost in that configuration.

**Why this class and why the triplet is sufficient for this wave.** Linux
origin records use a filesystem UUID plus a filesystem-defined export file
handle/fid; Linux treats duplicate or unusable UUIDs as non-decodable and
disables or falls back from features that depend on origin decoding. Asterinas
has no export-FH surface, so `st_dev` alone cannot provide the same guarantee.
The accepted middle field is the configured lower layer root directory's
inode number. It distinguishes the practical same-`st_dev` case when the two
configured roots have different inode numbers, while avoiding both a new VFS
interface and the v1 mistake of persisting a mount-local ordinal.

**Accepted residual limitation and Linux gap.** The middle field is a layer-
root fingerprint, not a durable filesystem identifier, and the wire stores no
path. If two current lower filesystems have the same `st_dev` and the same
configured-root inode number, the unique-match rule detects the ambiguity and
returns `None`: attribution remains safe, but durable `st_ino` continuity can
be lost. The rarer temporal case cannot be detected: after unmount, filesystem
A may be replaced by filesystem B with the same device value, configured-root
inode number, and object inode number, causing the old complete triplet to
match B. This complete-triplet collision is accepted as a known limitation for
the current wave. Linux's `(filesystem UUID, filesystem-defined file handle)`
is the recorded stronger behavior; the native triplet must not be described as
Linux-equivalent or universally collision-free.

### 5A.4 Correctness / Contract Items

#### C1. Valid readdir index versus facts used for `..`

**Disposition (user ruling 2026-08-04): NO STRUCTURAL REPAIR; DOCUMENT THE
WEAKER CONTRACT.** Directory copy-up creates upper directory authority while
retaining the lower stack and does not by itself change the visible real-entry
sequence, so it does not invalidate an otherwise valid readdir index. The
changes that can alter that sequence are opaque/whiteout transitions and
namespace mutations; those paths remain responsible for fine-grained index
updates or `NeedsRebuild`. Do not add a generation, fingerprint, or stored
`OverlayObjectFacts` snapshot for this item.

**Principle.** One `readdir` call must use a coherent namespace snapshot: the
cached real-entry sequence and the facts used to synthesize `..` must either
belong to the same build epoch or be proven compatible under a weaker,
explicit invariant.

**Current contract gap.** `ReaddirIndex` owns only the cached real entries and
their cookie/tombstone validity. `ensure_readdir_index` returns revalidated
scan facts after a rebuild, but returns the caller's current facts on the
`Valid` fast path. Those current facts are used only for synthesized head
entries, principally `..`; they are not evidence that the cached real entries
were built from the same facts epoch. Existing comments claim that stronger
same-epoch property and are false. The mechanical repair must document the
weaker split contract: index validity covers the visible real-entry sequence;
the returned/current facts feed the head entries under the same `DIR`
transaction. Any remaining wrong `..` identity is the deferred P1
overlay-parent-context gap, not a reason to pin a facts snapshot in the index.

#### C2. Complete bounded `EEXIST` retry for workdir temps

**Disposition (user ruling 2026-08-04, superseding the earlier deferral):
REQUIRED PRE-WAVE5 CORRECTNESS REPAIR.** A complete repair spans creation and
namespace-mutation paths, so it first requires one bounded Designer addendum
that freezes the shared request type, retry bound, call-site mapping, and
failure semantics. It then MUST be implemented across the full six-file scope
before wave5; no partial two-site repair is valid.

**Principle.** Workdir staging names are collision-resistant, not collision-
free. Every operation that creates/links/mknods a temp name must retry
`EEXIST` with a newly generated name up to one shared bound.

**Current defect.** The existing `create_workdir_temp_with_retry` is private
to `copyup/promote.rs` and only wraps `OverlayFs::create_workdir_temp`; it
cannot be reused by `mknod` or `link` paths. Single-attempt sites include the
special-node promotion leg and clear-empty removal. A complete census also
includes create-over-whiteout (`create` and `mknod`), link-over-whiteout, and
both whiteout-temp representations. Therefore the round-9 review's two-site
wording is not a sufficient global repair.

**Why this remains a correctness gap.** A residual collision can turn a valid
copy-up, create, link, whiteout, or rmdir into `EEXIST`; a random suffix only
reduces probability and is not a retry contract. The required design avoids an
opaque closure and moves one explicit request enum plus the shared bound to
`OverlayFs`/`copyup/workdir.rs`:

```rust
enum WorkdirTempRequest {
    Create { kind: InodeType, mode: InodeMode },
    Mknod { mode: InodeMode, node: MknodType },
    Link { source: Arc<dyn Inode> },
}

struct WorkdirTemp {
    name: String,
    inode: Arc<dyn Inode>,
}
```

The shared helper generates a fresh name on each attempt, retries only
`EEXIST` up to one bound, and returns the successful name/inode pair. The
required implementation packet MUST cover `copyup/promote.rs`,
`copyup/workdir.rs`, `dir/remove.rs`, `dir/create.rs`, `dir/link.rs`, and
`dir/whiteout.rs`; partial two-site repair is forbidden.

### 5A.5 Mechanical Repairs (All MUST Fix)

1. Update the stale `ensure_readdir_index` documentation to describe its
   `Result<OverlayObjectFacts>` return contract using C1's decided weaker split
   invariant (current facts for head entries; no same-build-epoch claim).
2. Replace `parent_fallback`'s false "single call site" statement; it is shared
   by five decision arms.
3. Remove/qualify `dir/remove.rs`'s "EVERY xattr-copy error" wording because
   invalid `XattrName` returns `EINVAL` before the best-effort policy branch.
4. Make `list_xattr_names` and `read_xattr_value` associated functions (or
   equivalent owner-local helpers) because neither uses `self`.
5. Replace side-effecting `Option::map` in `mount/build.rs` with explicit
   `if let`.
6. Remove the never-read `IdentityPolicy::layer_devs` field while retaining a
   construction-local table for `is_all_layers_same_fs` and
   `lower_layer_devs` derivation.

These are mechanical because the intended behavior and owner are unchanged.
They still matter: stale comments misstate error/cache contracts, unused state
suggests a false runtime dependency, and unused receivers/surprising map-side
effects obscure ownership.

### 5A.6 Superseded Round-9 Packet

`subagent-tasks/pre_wave5_design/pre_wave5_repair_r9_dispatch.md` was written
but never dispatched and is now **SUPERSEDED / MUST NOT DISPATCH**. Its
write-set lists only `readdir_index.rs`, `copyup/{workdir.rs,promote.rs}`,
`dir/remove.rs`, `metadata_security/xattr.rs`, `mount/build.rs`, and
`projection/identity.rs`, while its acceptance requires "every workdir-temp
creation" to retry. The actual census also reaches `dir/create.rs`,
`dir/link.rs`, and `dir/whiteout.rs`; the packet cannot satisfy its own
acceptance rule. It remains superseded after C2's promotion: a new bounded
Designer addendum and a new complete six-file implementation packet are
required. The old packet must not be narrowed or revived under another round
number.

### 5A.7 Required Next Sequence

1. Implement P3's accepted native triplet with unique
   `(device, lower-root-ino)` resolution and conservative ambiguity fallback.
   Keep the accepted complete-triplet collision risk and Linux UUID/FH gap in
   the durable record; do not widen this packet into a VFS/export-FH change.
2. Issue and accept one bounded C2 Designer addendum, then implement its
   shared typed request/retry contract across all six files listed in §5A.4.
   It must retry only `EEXIST` with a fresh name per attempt under one shared
   bound; it may not degrade to a random-suffix-only or partial repair.
3. Issue one bounded mechanical-repair packet for the six items in §5A.5. Its
   readdir documentation must express C1's weaker split contract; it must not
   add P1, P2, generation/fingerprint state, or other production behavior.
4. Complete the B traceability/deviation audits, then enter wave5's Checker-
   owned compile/lint lane. No build or runtime command belongs in this manual
   adjudication step.
5. Keep P1/P2 in the deferred register. A future repair requires an explicit
   user ruling; do not revive the old r9 packet and do not call the next
   action round 10.

## 5B. Wave5 Static Entry Stop Record (2026-08-04)

**Final evidence:**
`components/wave_05_compile_lint/pass_11_wave5_compile_lint_checker.md`,
continuation 03. The only authorized command was
`docker exec -w /root/asterinas codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo check -p aster-kernel --target x86_64-unknown-none'`.
It exits `101` with 15 errors and one warning at `7aabd029c`; the original
42-error / 16-warning receipt and both intermediate receipts are preserved.
No `make kernel`, `make check`, clippy, QEMU, xfstests, ktest, or Reviewer ran.

**Closed mechanical batch:** imports, local/re-export visibility, workdir
request re-export, slice conversions, direct `Result`/type/reference repair,
the `PositiveBinding` owner accessor, and lint renames are all amended. The
sole continuation-02 `Result<bool>` mismatch was repaired and is absent from
the final output. Do not reopen this batch absent a new Checker diagnostic.

**User decisions now binding for the follow-up packets (2026-08-04):**

1. **Root slot:** do not use `OnceLock`. Only mount construction writes
   `OverlayFs::root_inode`; it may replace the construction-local value more
   than once, provided a complete root is present before `root_inode()` is
   callable. Use a private construction/publication slot (the proposed shape
   is `Mutex<Option<Arc<dyn Inode>>>`); the getter clones a prepared root and
   performs no fallible work.
2. **Trait owner and forwarding:** `projection/inode.rs`, which declares
   `OverlayInode`, owns the sole `impl Inode` and sole `impl FileOps`.
   Functionality remains in its current Meso modules as ceiling-visible
   inherent helpers with an `_impl` suffix (for example `create_impl`); the
   two trait impls become small behavior-preserving forwarders and must not
   acquire, release, or reorder locks beyond the existing helper bodies.
3. **Mounting context:** add the VFS seam
   `FsCreationCtx::task_ctx() -> &Context` at `pub(in crate::fs)`. Overlayfs
   borrows it only during `FsType::create` for layer-root path resolution and
   immediate `credentials_dup()`; it neither stores `Context` nor implements
   P2 credential override.
4. **In-use claim:** add a dedicated VFS `Extension` group for an
   inode-owned `OverlayInuseSlot` with atomic token claim/release operations
   and an `InodeExt` accessor. Do not reuse the existing event-publisher or
   lock-context groups and do not create an overlay-global map. The existing
   `InodeClaimGuard` retains the real inode pin and releases its own token in
   `Drop`.
5. **Source link admission:** at `dir/mod.rs:193`, call the two-argument
   inherent permission gate with `AccessType::ReadOnly`. This is the frozen
   non-promoting source check before `link_source()` may trigger copy-up;
   `AccessType::Mutating` remains for affected namespace parents.

**Next scheduling boundary:** separately packet (a) the VFS seam for the
context accessor and `OverlayInuseSlot`, and (b) the Overlayfs root/trait/
permission repairs. No packet may introduce a different root carrier, store a
mount context, reuse an existing extension group, use a global claim map, or
change the `ReadOnly` source-admission choice. P1/P2 retain their earlier
deferred status and are not part of these repairs.

## 5C. Accepted Five-Item Designer Reconciliation (2026-08-04)

**Dispatch:** `task_designer_wave5_static_owner_reconciliation_20260804`,
archived at
`subagent-tasks/wave_05_static_repair_design/meso_08_static_owner_reconciliation_designer_dispatch.md`.

**Acceptance:** The Designer delivered and the main agent structurally accepted
`components/wave_05_static_repair_design/meso_08_static_owner_reconciliation_designer_spec.md`
and
`components/wave_05_static_repair_design/meso_08_static_owner_reconciliation_designer_validation.md`.
The acceptance verified the complete five-direction signature/visibility
mapping, one `impl Inode`/one `impl FileOps` target, existing-Meso helper reuse,
no new Overlay carrier/module/global map, and no production edit or validation
command. A first-draft `Option` slot ambiguity was returned to the Designer;
the accepted revision makes the accessor the sole lazy initializer and removes
the obsolete missing-slot branch.

**Purpose and boundary:** Freeze the already user-adjudicated code forms for
the root publication slot, single `OverlayInode` trait owner and thin
forwarders, mount-time `FsCreationCtx::task_ctx()`, inode-owned
`OverlayInuseSlot`, and non-promoting source link permission. This is a
bounded cross-Meso revision, not a new Meso, not a Creator pass, and not a
semantic redesign.

**Non-negotiable minimization:** the expected VFS production write-set is only
`fs_apis/{registry.rs,inode.rs,inode_ext.rs}`; Overlayfs must reuse the
existing `Mutex`, Extension/InodeExt group idiom, atomic operations,
`InodeClaimGuard`, and Meso helper bodies. The accepted slot uses a dedicated
third Extension group and `Acquire`/`Release`/`Relaxed` operations only for
token ownership; it adds no lock domain. The Designer made no `.rs` edit, ran
no command, designed no ktest, and scheduled no implementation. Separate VFS
and Overlayfs Creator packets may now be considered, but are not authorized by
this acceptance.

## 5D. Continuation-04 Static Check Dispatch (2026-08-04)

After the accepted coordination records were amended at `783c81041`, the main
agent dispatched
`task_checker_wave5_compile_lint_20260804_continuation_04` through
`subagent-tasks/wave_05_compile_lint/pass_11_wave5_compile_lint_checker_continuation_04_designer_reconciliation_dispatch.md`.
It authorizes exactly one attempt of the existing target-specific container
`cargo check` command, no production edit or repair, and a distinct preserved
evidence run. The accepted Designer reconciliation is documentation only, so
this continuation must not describe it as implementation or infer that the
previous 15-error result is repaired. `make`, Clippy, runtime, and xfstests
remain forbidden.

**Result:** Checker continuation 04 ran that one exact command at
`783c81041`; it exited `101` with the same 15 errors and one warning as
continuation 03. Receipt:
`components/wave_05_compile_lint/pass_11_wave5_compile_lint_checker.md` §9;
raw output:
`components/wave_05_compile_lint/run_evidence/20260804T_continuation_04_designer_reconciliation/run_20260804T_continuation_04_targeted_cargo_check.stdout_stderr.log`.
The five error categories now explicitly prove only that the accepted root,
trait, VFS, and permission designs have not been implemented. No new
mechanical repair exists, and no more continuation, build, lint, runtime, or
xfstests command is authorized without an implementation packet.

## 5E. Five Separate Creator Implementations (2026-08-04)

**User authorization:** Implement the five accepted forms separately, then
have the main agent review each exact production diff. The packet group is
`subagent-tasks/wave_05_static_owner_repair/`; every Creator is command-free,
cannot run git, and writes one ignored receipt under
`components/wave_05_static_owner_repair/`.

**Slicing and dependency:** pass 12 root publication (`P0-05`) changes
`mount/{superblock.rs,build.rs}`; pass 13 canonical trait ownership changes
the seven trait-body files; pass 15 in-use slot (`P1-35`) changes the two VFS
extension files plus `mount/claims.rs`. These three disjoint write-sets start
first. Pass 14 task context (`P0-01`, `P1-19`) waits for pass 12 because it
also changes `mount/build.rs`; pass 16 source link permission (`P1-28`) waits
for pass 13 because it changes the post-forwarder `dir/mod.rs` helper. The
main agent reviews each task before dependent dispatch and amends only all
accepted Rust changes together. A final Checker continuation uses the exact
previously verified target-specific container `cargo check` command.

**Pass 12 review:** Accepted. The exact `mount/{superblock.rs,build.rs}` diff
removes `OnceLock`, uses `Mutex<Option<Arc<dyn Inode>>>`, creates the root
outside the slot lock, publishes `Some(root)` before return, and makes the
getter clone only a prepared root. No new entity, lock-order change, command,
or out-of-scope edit was found; pass 14 is unblocked.

**Pass 15 review:** Accepted. The exact VFS `inode.rs`/`inode_ext.rs` plus
`mount/claims.rs` diff creates only the dedicated token slot and group3,
preserves group1/group2, initializes only through `overlay_inuse_slot()`, and
uses the frozen claim/release/observation orders. `InodeClaimGuard` retains
its inode pin and only CAS-releases its own token; no global map, mutex,
new lock domain, command, or out-of-scope edit was found.

**Pass 14 review:** Accepted. The exact VFS diff adds only
`FsCreationCtx::task_ctx() -> &Context<'a>` at `pub(in crate::fs)`. The two
already-written mount construction expressions are the complete consumer set;
they immediately resolve paths or duplicate credentials and retain no context.
No new type, carrier, lock, credential override, command, or out-of-scope edit
was found.

**Pass 13 review:** Accepted. `projection/inode.rs` now holds the sole
`impl Inode` and `impl FileOps` for the new `OverlayInode`; every sibling Meso
body remained in its original file as an exact `*_impl` helper. The follow-up
corrected stale ownership prose in `copyup/mod.rs`, `permission.rs`, and
`projection/inode.rs`; no behavior, signature, lock scope, or out-of-scope
production edit was introduced.

**Pass 16 review:** Accepted. The sole additional `dir/mod.rs` call now uses
`old_overlay.check_permission(AccessType::ReadOnly, Permission::MAY_WRITE)`
after the existing owner predicate and before `link_source()`. The target
parent's existing `Mutating` admission, target `DIR` guard, error conversion,
and copy-up ordering are unchanged.

**Five-pass amend:** The 13 accepted Rust files were amended to
`10cf627e2` (`Add pre-wave5 bounded overlayfs revisions`); only the live
`PASS_SLICING.md`, `SYSTEM_BLUEPRINT.md`, and this handoff remained modified
outside the commit.

**Checker continuation 05:** The packet
`pass_11_wave5_compile_lint_checker_continuation_05_static_owner_implementation_dispatch.md`
authorized the same one target-specific container command at `10cf627e2`.
It exited `101` with five errors and one warning; raw evidence and the full
classification are in `components/wave_05_compile_lint/pass_11_wave5_compile_lint_checker.md`
§10 and its referenced run directory. The command was not retried; no
`make`, Clippy, runtime, or xfstests command ran.

**Mechanical continuation repair:** The Checker proved three direct repairs:
the missing `AccessType` import in `projection/inode.rs`, the missing `Inode`
trait import in `metadata_security/permission.rs`, and the unused local in
`dir/mod.rs`. The main agent applied only those spelling/scope/lint changes
and amended them to `1378d502a`.

**Pass 17 ownership-order repair:** The user approved the frozen no-clone
ordering. A command-free Creator changed only `projection/entry.rs` and
`readdir_index.rs`: upper opacity is evaluated before either returning or
moving the hit, lower opacity is recorded before insertion and tested after
insertion, and inode `type_()` is read before the `Arc` enters the readdir
tuple. Exact-diff review accepted this as preservation of the existing opaque
barrier, merged-layer, and `d_type` contracts; it was amended to `90a5facf7`.

**Checker continuation 06:** The exact prescribed cargo check at `90a5facf7`
exited `101` with the three `E0382` errors absent. Its only errors were the
pre-existing `UpperWorkdirClaim` visibility mismatch exposed by
`OverlayFs::claims()` and consumed by the single `OverlayFs::workdir_root()`
resolver; the receipt records 17 warnings. This is direct interface
propagation, so the main agent widened only `UpperWorkdirClaim` to the existing
overlayfs ceiling and amended `36c30ac33`; no claim field, method, lifecycle,
lock, or call ordering changed.

**Checker continuation 07 / current result:** The same exact one-command
container cargo check at `36c30ac33` exited `0` in 8.54 seconds. Receipt:
`components/wave_05_compile_lint/pass_11_wave5_compile_lint_checker.md` §12;
raw output:
`components/wave_05_compile_lint/run_evidence/20260804T_continuation_07_claim_visibility/run_20260804T_continuation_07_targeted_cargo_check.stdout_stderr.log`.
It still emits the following **17 unresolved Rust warnings**. They are recorded
as warning/lint debt, not as a user-authorized cleanup:

| Root cause | Diagnostics and locations | Current boundary |
|------------|---------------------------|------------------|
| Published signature has a narrower return type | One `private_interfaces` warning: `UpperWorkdirClaim::identity()` is visible at the overlayfs ceiling, but its returned `OverlayUuid` remains `mount`-private (`mount/claims.rs:411` and `:64`). | Do not silence it with `allow`. A later bounded decision must choose whether `identity()` should narrow to its true consumers or `OverlayUuid` is intentionally published to the same ceiling; that is an API/owner visibility choice. |
| Facts/evidence are constructed but not consumed | Five `dead_code` diagnostics: `PositiveBinding::kind` (`projection/binding_cache.rs:67`), the `HiddenEvidence` payload of both `NegativeBinding` barrier variants (`:123`, `:125`), `HiddenEvidence::{layer_index, real_inode}` (`:136-138`), and VFS `OverlayInuseSlot::is_claimed()` (`fs_apis/inode_ext.rs:54`). | These are not automatically removable: the binding payloads preserve barrier/evidence state, while the VFS method is part of the new slot API. A cleanup must first prove every actual and intended consumer/disposition. |
| Completed cross-Meso seam still expects `dead_code` | Eleven `unfulfilled_lint_expectations` warnings: `mount/superblock.rs:143,155,168`; `overlayfs/mod.rs:18`; `mount/policy.rs:210`; `projection/binding_cache.rs:81,150,190,267`; `projection/entry.rs:76`; and `projection/mod.rs:229`. | Each `#[expect(dead_code)]` was a forward-reference marker when its seam was unused. The Wave-4 consumers now use those seams, so Rust correctly warns that the expectation no longer fires. Removing the stale attributes is likely mechanical but must be a separately scoped lint-cleanup pass; do not convert them to `allow` or claim lint acceptance first. |

No `make kernel`, `make check`, Clippy, runtime, or xfstests command has run,
so the Wave5 static lane has **only cargo-smoke acceptance**, not lint
acceptance. The next static lane action remains a separate Checker packet for
`make kernel`; the warning register must stay attached to any later `make
check`/lint-cleanup scheduling decision.

## 5F. Post-Smoke Warning Decisions (2026-08-04)

The user made the following binding dispositions for the continuation after
the warning register was recorded:

1. **`OverlayUuid` visibility:** `OverlayUuid` is an overlayfs-wide component,
   not a mount-private implementation detail. The next bounded lint-cleanup
   implementation must raise it to the overlayfs visibility ceiling, matching
   `UpperWorkdirClaim::identity()`; it must not narrow that accessor or add an
   `allow(private_interfaces)` suppression. This is visibility propagation,
   not a change to UUID representation, construction, or claim lifecycle.
2. **Stale expectations:** the 11 unfulfilled `#[expect(dead_code)]` markers
   are mandatory lint-cleanup work. Remove them while resolving the future
   lint lane; do not replace them with `allow`. This decision does not claim
   that `make check` has run or authorize a broad warning cleanup.
3. **Open evidence/API questions:** before disposing of
   `PositiveBinding::kind`, either `NegativeBinding` evidence payload, or
   `OverlayInuseSlot::is_claimed()`, inspect their construction, matching, and
   intended lifecycle paths. In particular, a syntactically unread
   `Arc<dyn Inode>` may still be the lifetime pin promised by `HiddenEvidence`;
   it must not be removed merely to satisfy `dead_code`. The relevance of
   `is_claimed()` to the mount claim protocol remains an explicit diagnosis
   item, not an assumed cleanup.
4. **Hidden evidence disposition:** retain the complete `HiddenEvidence`
   payload, including both `layer_index` and `real_inode`; do not add a fake
   consumer and do not shrink the carrier. Its retained evidence/provenance
   and `Arc` lifetime-pin roles are intentional even where the current wave
   has no field read. The later bounded lint-cleanup must place narrowly
   targeted, reason-bearing `#[expect(dead_code)]` annotations on the
   diagnostics caused by this intentional unread payload. It must not use
   `allow`, and this disposition does not authorize a behavioral revalidation
   change.

### 5F.1 Warning-payload diagnosis (2026-08-04)

The requested code/design cross-check produced the following results. These
are diagnostic findings, not yet a user decision to alter the binding or VFS
API shapes.

1. **`PositiveBinding::kind` is currently a redundant, and potentially stale,
   snapshot.** Every construction copies `facts.kind` into the binding, while
   all current consumers either test only `Binding::Positive(_)` or extract
   the inode; classification consumers use `OverlayInode::facts_snapshot()`.
   More importantly, copy-up replaces the shared inode facts without
   replacing already-published bindings: a formerly lower-only directory may
   become `Merged` when an upper directory is created while lower inputs stay
   present. Reading the immutable binding field after that transition could
   yield the old `Single` classification. Do not invent a read merely to
   silence the warning. The likely correct follow-up is a small Designer
   revision that removes `kind` from `PositiveBinding` and makes the
   inode-owned facts the sole classification source, with matching design-doc
   correction; this must be decided before a Creator changes the carrier.
2. **Hidden variants are active, but their evidence has two distinct roles.**
   Layer lookup and namespace mutation construct `HiddenByWhiteout` and
   `HiddenByOpaque`; create/link/rename/remove consume their discriminant to
   choose the visibility recipe, while VFS receives `ENOENT`. No current code
   reads `HiddenEvidence.layer_index` or borrows `real_inode`. Nevertheless,
   retaining `real_inode: Arc<dyn Inode>` keeps the barrier object pinned until
   the cached negative binding drops, which is an intentional `Drop`-lifetime
   effect not visible to `dead_code`; it must remain. `layer_index` has no
   present read or destruction role, but the documents reserve it for barrier
   provenance / revalidation while the current conservative
   `revalidate_absent` always returns `false`. The user decided to retain the
   complete payload rather than fabricate a read or reduce the carrier; the
   future bounded lint-cleanup will document that intentional state with
   narrow, reason-bearing `#[expect(dead_code)]` annotations.
3. **`OverlayInuseSlot::is_claimed()` is not a missing mount safety check.**
   Mount construction claims upper then workdir with
   `try_claim(token)`'s `compare_exchange(0, token, ...)`; that CAS is the
   atomic test-and-acquire and returns `EBUSY` on conflict. A pre-check with
   `is_claimed()` would have a TOCTOU race, while a post-check is redundant
   and does not establish that the current mount owns the token. The correct
   mount observer is `UpperWorkdirClaim::has_exclusive_claim()`, which uses
   `is_claimed_by(identity)` for both pinned inodes (currently an unused
   future audit seam). No call site invokes the any-owner `is_claimed()`.
   Unless a concrete non-admission observer is specified, removing this VFS
   method and its Designer-spec row is the minimal lint-cleanup direction.

### 5F.2 Continuation-08 Static-Lane Stop (2026-08-04)

The post-`pass_18_warning_cleanup` Checker continuation completed its exact
container preflight, target-specific cargo smoke, and full kernel build:

- `cargo check -p aster-kernel --target x86_64-unknown-none`: **PASS** (0).
- `make kernel`: **PASS** (0).
- workspace Clippy: **FAIL** (101, 22 lint errors).

The first Clippy diagnostic is the explicitly deferred VFS method
`OverlayInuseSlot::is_claimed()` (`fs_apis/inode_ext.rs:54`). It was neither
suppressed, removed, redesigned, nor given a synthetic caller. Additional
diagnostics include structural constructor-field ordering and API/carrier
concerns (`too_many_arguments`, `type_complexity`, and `boxed_local`), as well
as separately packetable mechanical candidates. Per the user-directed
mechanical-only lint boundary, no source repair is authorized until the VFS
deferred decision and the structural/API dispositions are supplied.

The Checker did not run rustfmt or the process-local-wrapper `make check`.
Its receipt is
`components/wave_05_compile_lint/pass_11_wave5_compile_lint_checker.md` §13;
raw Clippy evidence is in
`components/wave_05_compile_lint/run_evidence/continuation_08_warning_cleanup/`.

### 5F.3 User-Directed Mechanical Continuation (2026-08-04)

The user clarified that the §5F.1 direction for the unconsumed any-owner
observer is binding: `OverlayInuseSlot::is_claimed()` was deleted rather than
given a synthetic consumer. The CAS admission (`try_claim`), guard release,
and token-specific observation (`is_claimed_by`) are unchanged. The user also
authorized the two field-initializer order repairs and all non-documentation
mechanical Clippy repairs, while explicitly deferring documentation-only lint
to Wave6.

The accepted `pass_19_post_clippy_mechanical_cleanup` changed only the eight
recorded Rust paths. Checker continuation 09 then passed the exact
target-specific container cargo smoke and `make kernel`; workspace Clippy
failed only with 12 residual diagnostics: nine user-deferred documentation
items and the three pending API/representation questions,
`MountPolicy::assemble` `too_many_arguments`, `BindingCache::entries`
`type_complexity`, and `IdentityPolicy::new` `boxed_local`. All prior
non-documentation mechanical diagnostics, including `is_claimed()`, are absent.
The continuation did not run rustfmt or `make check`; both wait on the three
pending dispositions and the Wave6 documentation cleanup. Receipt:
`components/wave_05_compile_lint/pass_11_wave5_compile_lint_checker.md` §14;
raw evidence:
`components/wave_05_compile_lint/run_evidence/continuation_09_post_clippy_mechanical/`.

### 5F.4 User-Approved `boxed_local` Continuation (2026-08-04)

The user approved the `IdentityPolicy::new` input repair after ownership scope
confirmation. The bounded Designer continuation freezes a borrowed
`&[(u64, DeviceId, u64)]` construction input: `mount/build.rs` has no
post-call use of its construction-local `layer_devs`, while `IdentityPolicy`
copies the relevant lower tuples into, and retains only,
`Box<[LowerLayerIdentity]>`. The Creator changed only that constructor
parameter and its sole `&layer_devs` caller; no carrier, lock, lifetime,
allocation, construction order, identity semantics, VFS surface, test, or
`legacy_fs.rs` change entered the continuation.

Checker continuation 10 passed the exact target-specific cargo smoke and
`make kernel`, then stopped at workspace Clippy (exit 101) with eleven
diagnostics only: the nine user-deferred Wave6 documentation items,
`MountPolicy::assemble` `too_many_arguments`, and
`BindingCache::entries` `type_complexity`. `IdentityPolicy::new`
`boxed_local` is absent. The receipt is
`components/wave_05_compile_lint/pass_11_wave5_compile_lint_checker.md` §15;
raw evidence is
`components/wave_05_compile_lint/run_evidence/continuation_10_identity_boxed_local/`.
No rustfmt, `make check`, runtime, or xfstests command ran.

### 5F.5 User-Approved Policy and Binding Lint Continuation (2026-08-04)

The user approved the two remaining non-documentation Clippy repairs after
the Designer froze their representation-only forms. Creator pass 21 changed
only `mount/policy.rs`, `mount/build.rs`, and `projection/binding_cache.rs`:
`MountPolicy::assemble` now takes a non-escaping construction-local
`&OverlayMountOptions` and copies only `uuid_mode`,
`is_default_permissions`, and `xino_mode`; its other five inputs keep their
individual ordering. The cache remains `parent -> name -> binding` behind the
same `RwMutex`; `BindingsByName` and `BindingsByParent` are private aliases
only. No owner, lock, allocation, cache key, cache operation, lifecycle,
test, VFS, `legacy_fs.rs`, or documentation/comment source change entered the
pass.

Checker continuation 11 passed the exact target-specific cargo smoke and
`make kernel`, then stopped at workspace Clippy (exit 101) with exactly the
nine user-deferred Wave6 documentation diagnostics: seven at
`mount/build.rs:44-50` and two at `dir/remove.rs:76-77`.
`MountPolicy::assemble` `too_many_arguments` and
`BindingCache::entries` `type_complexity` are absent; the earlier
`IdentityPolicy::new` `boxed_local` remains absent. The receipt is
`components/wave_05_compile_lint/pass_11_wave5_compile_lint_checker.md` §16;
raw evidence is
`components/wave_05_compile_lint/run_evidence/continuation_11_policy_binding_lint/`.
No rustfmt, `make check`, runtime, or xfstests command ran. Wave6 exclusively
owns the remaining documentation cleanup, after which rustfmt and `make
check` may be scheduled.

## 6. Live File Discipline

- **This file is the live handoff for:** 2026-08-03 Creator Pass Slicing tenure (Phase 4, from 2026-08-03 onward).
- **Update rule:** Update this same file in place as Creator waves are dispatched, accepted, rejected, or escalated; continuation events go under the component evidence area, not here.
- **Supersedes / Replaces:** `20260801-state-cleanup-designer-prep_main_agent_handoff.md` (marked SUPERSEDED 2026-08-03).
