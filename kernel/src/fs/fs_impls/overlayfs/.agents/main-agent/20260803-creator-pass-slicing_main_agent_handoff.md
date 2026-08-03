<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: 2026-08-03 Creator Pass Slicing (Phase 4)

**Date / Time:** 2026-08-03
**Status:** `Active — Phase 4 in progress: Creator pass slicing recorded (7 passes, 3 serial foundation waves + 4-way parallel leaf wave); workflow amended (no per-pass Checker; Reviewer is the only pre-code-completion gate; single meso-integration xfstests gate remains). Next: dispatch Wave 1 (pass_01) then Wave 2 (pass_02), then the seams pass, then the 4-way parallel leaf wave.`

## 1. Global State Pointer

- **Current Active Wave / Pass:** None dispatched yet. Phase 3 (Designer) is **closed**; all 6 basic-wave Designer contracts are `Specified` and accepted (`mount_resource_policy` 9/6, `visibility_projection_identity` 12/2, `merged_directory_index` 4/1, `copyup_authority_file_views` 17/6, `metadata_security_xattr_policy` 4/4, `namespace_mutation_whiteout` 11/1); meso 07 is deferred-only (0/4). The prior tenure's handoff
  (`20260801-state-cleanup-designer-prep_main_agent_handoff.md`) is marked **ENDED / SUPERSEDED**; this file is the single live handoff.
- **Blueprint Updates Made:** Yes (2026-08-03): Phase 3 marked closed; Phase 4 opened with the pass-slicing decision; the pipeline index's per-pass Checker column amended to `not scheduled (workflow amendment)`; the workflow amendment and the seams-pass rationale are recorded in `PASS_SLICING.md`.
- **Accepted baseline:** Phase 0-2 accepted; fresh Architect topology owns all 81 formal Micro IDs; Stage-D scope 57 `需要实现` / 24 `暂不实现` unchanged; meso 01-06 contracts `Specified`; meso 07 deferred-only.

## 2. Pass Slicing Decisions (Creator Wave — file-level dependency graph, max parallelism)

**Workflow amendment (user-directed 2026-08-03):** the test flow is xfstests-integration-only. There are **no Creator-synced per-pass Checker passes** anymore. The only runtime validation gate is the **meso-integration xfstests Checker** scheduled after implementation + Reviewer stabilize. Before the code is complete, the **Reviewer** is the only quality gate (static review; the user explicitly requests the pre-checker structural-audit role of PROTOCOL §1 rule 16). Creator passes are **command-free** and receive **no per-pass compile preflight** — compile evidence is owned by the integration Checker. This amends PROTOCOL §1 rule 5 for this wave by user direction; PROTOCOL.md text is not edited unless the user confirms a permanent amendment.

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

- **Reviewer (Wave 5):** one Reviewer pass per meso (or a grouped review wave listing every pass + covered-micro union), executed after the Creator receipts land. Reviewer is the **first and only static gate**; it may directly apply line-level non-functional fixes and returns structural findings to the owning Creator. No Checker evidence exists yet, so the Reviewer's compile-check doubt clause (REVIEWER.md §16) routes back to the Creator lane (or, at the end, the integration Checker's compile run).
- **Integration Checker (Wave 6):** one meso-integration Checker pass (or a small bounded set) per the six Designer validation contracts: repository-entry compile + full kernel build + the overlay xfstests suite (`overlay/001`-`078`, `100`, `101` per `full.list`), reporting mapped/observed/not-run per contract. This is the **only** runtime validation. Deferred rows from the meso-01..06 validation contracts (e.g., the meso-01 mount group's runtime deferral, meso-02/03 readdir-merged pickup, meso-04/05/06 integration obligations) are resolved here.


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
- **Next: Wave 4 (final implementation wave)** — four leaf mesos, 16 per-file creators: `readdir_index.rs` (meso-03, 4 micros), `copyup/*` 5 files (meso-04, 17 micros), `metadata_security/*` 4 files (meso-05, 4 micros), `dir/*` 6 files (meso-06, 11 micros). Batched ≤6 concurrent. Wave-4 must declare `CopyUpTransition`/`ReaddirIndex`/`OverlayXattrPolicy`/`WhiteoutCache` at the overlayfs ceiling and honor the `replace_facts`/`alias_key` serialization obligation.

## 4. Explicit Agent-Level Decisions

1. **Workflow amendment (user-directed):** Creator-synced per-pass Checker passes are eliminated for this wave; Reviewer is the only pre-code-completion gate; the meso-integration xfstests Checker is the single runtime gate; Creator passes are command-free with no per-pass compile preflight. Recorded in `PASS_SLICING.md`; PROTOCOL §1 rule 5 remains unedited pending user confirmation of a permanent amendment.
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

1. **Dispatch Wave 1:** create `subagent-tasks/mount_resource_policy/pass_01_mount_resource_policy_creator_dispatch.md` (task id `task_creator_pass_01`; role Creator; kind implementation; risk High; write-set per §2.2; capabilities `can_edit` only — no compile) and dispatch via `$ovfs-subagent`. Artifact: `components/mount_resource_policy/pass_01_mount_resource_policy_creator.md`. Accept structurally against the Creator template. **Every Creator packet must carry the legacy-reference boundary (§4 decision 6):** the only permitted reference to `legacy_fs.rs` is the `OverlayFsType`/`register()` registration wiring; all other legacy content is forbidden; design sources are the Designer specs, `designdoc/`, and `priors/`.
2. **Dispatch Wave 2** (`pass_02`, same pattern, `subagent-tasks/visibility_projection_identity/`), only after pass_01 is accepted (shared `mount/superblock.rs` + `mount/build.rs`).
3. **Dispatch Wave 3** (`pass_03_shared_carrier_seams`) after pass_02 acceptance. If any recorded widening is refused, do NOT parallel-dispatch Wave 4; escalate to the user.
4. **Dispatch Wave 4 — all four leaf passes in parallel** (`pass_04`..`pass_07`) after pass_03 acceptance; write-sets are disjoint by construction. Do not wait between them.
5. **Reviewer wave:** after all seven Creator receipts are structurally accepted, dispatch per-meso Reviewer passes (or one bounded review wave per meso enumerating the pass + covered-micro union). Reviewer is the only static gate; there are no per-pass Checker receipts to consume.
6. **Integration Checker:** after Reviewer acceptance, schedule the meso-integration xfstests Checker (compile + full kernel build + overlay suite) per the six Designer validation contracts; record evidence under each component directory.
7. **Deferred ledger items carried from the Designer tenure** (already recorded in the superseded handoff §11.4): meso-02 `xino_mode` publication gap (default Auto), WL Mutex-vs-RwMutex, meso-02 `nlink==1` gate, `layer_devs` policy input — none block Wave 1-4 dispatch.

## 6. Live File Discipline

- **This file is the live handoff for:** 2026-08-03 Creator Pass Slicing tenure (Phase 4, from 2026-08-03 onward).
- **Update rule:** Update this same file in place as Creator waves are dispatched, accepted, rejected, or escalated; continuation events go under the component evidence area, not here.
- **Supersedes / Replaces:** `20260801-state-cleanup-designer-prep_main_agent_handoff.md` (marked SUPERSEDED 2026-08-03).
