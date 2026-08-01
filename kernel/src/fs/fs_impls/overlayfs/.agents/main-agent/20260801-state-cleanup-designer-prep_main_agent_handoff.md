<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: 2026-08-01 State Cleanup & Designer Prep

**Date / Time:** 2026-08-02 (updated in place; session record 2026-08-02 appended)
**Status:** `Designer wave COMPLETE (2026-08-02): Phase 3 closed — meso 01-06 basic-wave contracts Specified; meso 07 deferred-only (0/4); next: Phase 4 pass slicing + Creator/Checker loops`

## 1. Global State Pointer

- **Current Active Wave / Pass:** None (no implementation pass active). Phase 3
  (Designer) is in progress per `SYSTEM_BLUEPRINT.md`: 3 of 7 contracts
  accepted (`mount_resource_policy`, `visibility_projection_identity`,
  `merged_directory_index`, `copyup_authority_file_views`,
  `metadata_security_xattr_policy`); **meso 03/04/05 revision-01 accepted
  2026-08-02**; **meso-01 default_permissions publication revision in flight**
  (closes meso-05 §8 escalation 1); **next main-agent action: dispatch the
  meso 06 Designer wave**.
- **Blueprint Updates Made:** Yes. This session (a) closed the lock-topology
  discussion and marked `UPPER` (and possibly `WL`) as `可能清理` cleanup
  candidates deferred to the Designer/Creator stages, (b) recorded that Phase 3
  Designer dispatch is next, and (c) re-confirmed that Phase 0-2 and the
  57/24 Stage-D classification (after the 2026-08-01 P2-11 amendment) are unchanged. `PASS_SLICING.md` unchanged (no
  new pass slices).
- **Accepted baseline:** Phase 0-2 accepted; fresh Architect topology owns all
  81 formal Micro IDs; Stage-D scope `57 需要实现 / 24 暂不实现` unchanged.

## 2. Session Record (2026-08-01)

1. Read the full intake (README, PROTOCOL, SYSTEM_BLUEPRINT, PASS_SLICING,
   latest handoff, seven Meso maps, Micro inventory, B/C drafts, Stage E/F).
2. Verified the five-lock design against `/home/ayd/linux` (7.2.0-rc3);
   the confirmed facts and sources are recorded in §4 and stay as design
   context for the Designer/Creator stages.
3. Analyzed `UPPER` as a lock level and presented the decomposition.
   **User ruling:** no separate convergence-list review is needed (the design
   documents already covered it); `UPPER` (and possibly `WL`) are marked
   `可能清理` and are left to the Designer/Creator stages to resolve; no
   convergence list and no artifact repair are created now.
4. Cleaned the old state (see §3) so later sessions cannot misread a closed
   tenure as live work.


## 2A. Session Record (2026-08-02)

1. **meso 03 + meso 04 Designer waves dispatched in parallel** (user-directed):
   packets under `subagent-tasks/merged_directory_index/` and
   `subagent-tasks/copyup_authority_file_views/`; both write-sets disjoint;
   both carried the §9A checklist and carrier-extension boundaries.
2. **meso 03 initial spec + validation produced and structurally accepted**;
   **user review raised 6 points**; rulings: (a) module restructure to a
   single flat file `readdir_index.rs` (no nested mod level; fallback two flat
   siblings only); (b) `version` removed (design docs BC-3 §25.3/§30, BC-2 §23,
   stageF §195; supersedes the meso-03 map's "version state" clause — recorded,
   map not edited); (c) opaque-toggle decision contract recorded for meso 06;
   (d) `ReaddirCookie` naming kept (Linux getdents / BC-3 §27 source); (e)
   tombstone scheme with `Weak<OverlayInode>` tombstones, eager compaction at
   `tombstone_count >= live_count` (amortized-optimal, c=1), strict
   revive-vs-create.
3. **meso 03 revision 01 (bounded continuation) accepted 2026-08-02** after
   structural verification: single flat module; renames applied; no `version`
   field/bumps; `validity` two-state kept; I9 single-instance/no-snapshot
   invariant; tombstone enum `Visible`/`Tombstone`; opaque contract; procfs-slot
   return semantics confirmed; validation contract untouched except a scoped
   sync note. No escalations.
4. **meso 04 spec + validation produced 2026-08-02** (17/6); structurally
   accepted in the initial pass; **formal user acceptance pending** — the next
   review target.
5. **Ledger fixes:** stale `56/25` references in §1/§7 corrected to `57/24`
   (the 2026-08-01 `P2-11` amendment is authoritative). `PASS_SLICING.md`
   unchanged (no pass slices created).
6. **meso 04 review + revision 01 (2026-08-02):** user review ruled (a)
   module `authority/` → `copyup/` (`copyup.rs` → `promote.rs`, `file_view.rs`
   deleted); (b) `RealFileView` eliminated — delegation on `OverlayInode`,
   `open()` returns `None` after EROFS/promotion side effects, per-call
   re-resolution follows copy-up (Linux `ovl_real_file_path`, file.c:128-171;
   VFS evidence: `InodeHandle` pins the mount via `Path -> Arc<Mount>`);
   mmap promotion anchored at write-intent open with a recorded VFS-seam
   dependency; (c) `WriteIntent` removed — promotion scope decided under `CUL`
   by inspection (top-down ancestor walk terminating at root, per-branch
   multi-parent, existence resolved by the walk); (d) `CopyUpRecipe` folded
   into `promote()`; (e) `CopyUpTransition { publication_parent, name, phase }`
   (field `copyup_transition`, hook `record_copyup_transition`); no agent-noun
   type names. **meso 04 revision 01 accepted 2026-08-02** — `Specified`.
7. **Board:** 4 of 7 Designer contracts `Specified`; meso 05
   `metadata_security_xattr_policy` dispatch is the next action (consumes the
   accepted meso-04 seams `ensure_upper_authority` / `select_real_inode` and
   replaces meso-04's `is_overlay_private_xattr_name` with its classification
   policy).
8. **meso 05 review + revision 01 (2026-08-02):** user rulings — (1) promote
   `AccessClass` to the shared overlayfs-level `AccessType { ReadOnly, Mutating }`
   (crate root, cross-meso adoption note); (2) remove the `OverlayPrivateXattr`
   payload enum — `XattrClass` payload-less, `is_private` is the
   judgment/supersession method, the known-private table becomes a
   module-private const + insertion-point table; (3) split the fused
   `authorize` into ONE public entry + ≥2 private stage helpers
   (`check_permission(access, perm)` = `check_local_permission` (stage A:
   EROFS + projection DAC) → unless `default_permissions` →
   `check_real_permission` (stage B: `ensure_upper_authority` copy-up inside +
   creator-credential real check)); (4) light surface — 1 public + 2 pipeline
   privates + 1 `delegate_to_real` helper, no other machinery; real-permission
   self-evaluation evidence (ext2/ramfs) recorded and applied. **meso 05
   revision 01 accepted 2026-08-02** — `Specified`.
9. **meso-01 bounded revision dispatched 2026-08-02:** publish the parsed
   `default_permissions` option onto `MountPolicy` (field + `assemble` wiring +
   `is_default_permissions()` accessor, single representation) to close
   meso-05 §8 escalation 1; the accessor signature must match meso-05's frozen
   consumption (`fs.policy().is_default_permissions()`, stage-B skip only per
   BC-5 §49). Consistency with meso-05 verified at acceptance.
10. **meso-01 revision 04 accepted 2026-08-02 (Chandrasekhar):** `MountPolicy`
    gains `is_default_permissions: bool` (single representation, immutable
    snapshot); `assemble` takes it as the 7th param from
    `OverlayMountOptions::is_default_permissions`; accessor
    `pub(super) fn is_default_permissions(&self) -> bool` matches meso-05's
    frozen consumption exactly. **meso-01/05 inconsistency resolved**
    (user-directed sync annotation): meso-05 spec §1 and §8 item 1 annotated
    CLOSED (accessor now published); skip semantics unchanged (BC-5 §49).
11. **meso 06 Designer dispatched 2026-08-02 (Pasteur):**
    `namespace_mutation_whiteout` (11/1) — consumes all accepted contracts
    (01/02/03/04/05); packet includes the user-directed **page-cache
    forwarding research** deliverable (copy-up real-inode relocation →
    re-targeting; Asterinas page_cache()/mmap VMO path vs Linux no-overlay-
    page-cache / ovl_mmap / lower-mapping non-coherence; re-targeting contract
    + VFS dependencies; no meso-04 edits). Next after acceptance: meso 07.
12. **meso 06 review rulings + revision 01 (2026-08-02):** module
    `mutation/` → **`dir/`** (Linux dir.c alignment; no `publication.rs`);
    `publish_new_binding`/`publish_hidden_binding`/`publish_removed_name`/
    `publish_rename`/`maintain_index_*` **dissolved** — recipes compose owner
    seams inline (meso-02 `BindingCache`+`project_new_upper`; meso-03
    `readdir_index_insert`/`readdir_index_remove`/`invalidate_readdir_index`
    decision seams, Creator-implemented per the consumption-seam pattern, no
    meso-03 revision); `conservative_invalidate` → **`invalidate_stale_cache`**
    (single Case-13 reconcile entry); self-declared `read_only_gate` deleted
    (meso-05 `check_permission(Mutating)` admission); `lock_dir_transaction`/
    `lock_parent_dir_transactions`; `WhiteoutCache::{take,store}`; `upper_parent`
    kept; `WhiteoutRepresentation` derived from meso-01 capabilities (no
    runtime probe); **WL Mutex-vs-RwMutex deferred** to after all mesos (kept
    `Mutex<WhiteoutCache>`). Whiteout-cache semantics grounded in Linux
    `dir.c:77-119` (`ofs->whiteout` + `whiteout_lock` + `no_shared_whiteout`).
13. **meso-01 revision 02 (Chandrasekhar, 2026-08-02):** `UpperFilesystemCapabilities`
    gains `can_mknod_char` (post-claim workdir char-0:0 probe) + `can_store_private_xattr`
    accessor; whiteout-capability mount gate frozen (writable overlay requires
    at least one whiteout form else `EOPNOTSUPP` before commit). Closes the
    meso-06 whiteout-representation dependency. **meso 06 revision 01 and
    meso-01 revision 02 both structurally accepted; meso 06 formal acceptance
    pending user ruling.**
14. **meso 06 accepted 2026-08-02 (user ruling):** revision 01 `Specified`
    (dir/ module, dissolved publish helpers, invalidate_stale_cache, lock_*,
    take/store, WhiteoutRepresentation from meso-01 capabilities). meso 07
    dispatched (Pasteur/Ramanujan) with the user-directed page-cache research;
    initial + revision-01 produced (origin contract, read-side st_ino-stability
    leg, naming fixes).
15. **meso 07 TOPOLOGY AMENDMENT (2026-08-02, user-directed):** the origin/
    lower-id record has **no runtime owner** — it is generated by meso-04's
    copy-up publication (persisted `trusted.overlay.origin` xattr) and consumed
    by meso-02's identity projection (st_ino stability + xino mask). Ruling:
    (a) the record becomes a **meso-02 submodule** (`projection/lower_id.rs`,
    type **`LowerIdRecord`**); (b) meso 07 becomes **deferred-only (0/4)** —
    the future index feature is a **separate small copy-up-adjacent module
    concept**, not the current meso-07; (c) `P1-07` ownership moves to meso-02
    (scope 12/2; total 57/24 preserved). Dispatched: meso-02 revision
    (LowerIdRecord + IdentityPolicy consumption), meso-04 revision (publication
    calls the meso-02 store seam; object_id keeps lower-derived identity),
    meso-07 revision (deferred-only disposition).


## 3. State Cleanup Completed

- **Superseded banner added** to `20260724-p0-p1-design-tracking_main_agent_handoff.md`;
  it is now explicit historical context, not the live handoff.
- **This file is the single live handoff.** (Created 2026-08-01 as
  `20260801-lock-topology-convergence_main_agent_handoff.md`, then renamed in
  place to its current name because the lock-convergence phase is closed; the
  rename is part of the cleanup, not a new tenure.)
- **Board mirrored:** `SYSTEM_BLUEPRINT.md` now carries the lock-topology
  closure note and the `可能清理` marker (see §5); Phase 3 remains
  `Not started` with a recorded next action.
- **No pending runtime:** no open packets, no active passes, no stale lock on
  the command lane; `tmp/` holds only archival scratch (protocol report,
  design-atlas), `subagent-tasks/` holds only the two completed/archival
  packets (architect dispatch, old-baseline checker).

## 4. Confirmed Design Facts: The Five Locks vs Linux (Designer/Creator context)

- **Operation-path lock order:** `DIR -> CUL -> INODE -> WL -> UPPER`, plus
  `MOUNT` (level 1, exclusive during mount transition) and `IU` (out-of-band
  mount-time upper/workdir claim; `AtomicBool` + wait/claim, not a nested
  mutex level). These are Asterinas-specific domains; each level's role must
  trace to a Linux counterpart or a verified Asterinas substrate gap.
- **`DIR` = the role of Linux's VFS-held parent `i_rwsem`, realized as an
  Asterinas per-overlay-directory `ostd::sync::Mutex`.** Linux overlayfs has
  no per-directory lock of its own; it runs inside the VFS-held parent
  `i_rwsem` window (`include/linux/fs.h:816`; `fs/namei.c:1935` lookup shared;
  `fs/readdir.c:103` readdir shared; `fs/namei.c:2918, 3755-3829` mutation
  exclusive; `fs/overlayfs/namei.c:1382` `ovl_lookup()` inside that window;
  `fs/namei.c:3211-3241` `lookup_one_unlocked()` self-manages real-parent
  locks). Asterinas VFS holds no parent-dir lock across inode ops, hence the
  new `DIR` domain (priors `MICRO_FEATURE_INVENTORY.md`, Architect Notes #1).
- **Counterparts:** `INODE` ≈ `ovl_inode->lock`
  (`fs/overlayfs/ovl_entry.h:172`); `WL` ≈ `ovl_fs->whiteout_lock`
  (`ovl_entry.h:91`); `IU` ≈ `I_OVL_INUSE` inode flag
  (`fs/overlayfs/util.c:1014-1047`, in-memory, not an xattr); `CUL` and
  `UPPER` have no direct Linux counterpart. Linux overlayfs owns exactly two
  locks total.
- **Open semantic points for Designer contracts:** (a) `DIR` as `Mutex`
  serializes even concurrent lookups in one overlay directory, whereas Linux
  `i_rwsem` allows reader concurrency — the Designer must freeze the choice
  (`Mutex` vs sleep-capable `RwMutex`); (b) Asterinas
  `Dentry::lookup_child()` carries the VFS children-cache `RwMutex` into
  `inode.lookup()` — a non-Overlay guard at the inlet, a lock-order/re-entry
  hazard for the future VFS lookup reservation.

## 5. UPPER / WL Status (user-directed)

- `UPPER` (and possibly `WL`) are marked **`可能清理` (cleanup candidates)**.
- This is a marker only: no convergence list, no artifact repair, no topology
  edit is active now. The decision is deferred to the **Designer and Creator
  stages**, which may resolve or flag it inside their meso contracts.
- Design context preserved for those stages: `UPPER` as a mount-scoped
  level-6 mutex is a placeholder tag (47 of 81 Micro IDs carry it) rather than
  a defended invariant; the plausible protections decompose into concerns
  already owned elsewhere — namespace entry-set consistency (`DIR` + upper
  filesystem's own locks), copy-up workdir staging (workdir unique-naming +
  `CUL`/`INODE`), whiteout/opaque/impure markers (`WL`/`INODE`), real file I/O
  (no Overlay lock). Linux overlayfs has no upper lock; workdir temp naming is
  uniqueness-based (`fs/overlayfs/dir.c:63-66`), not lock-based. `IU` already
  covers mount-time upper/workdir exclusivity. This context does not mandate
  any specific outcome.

## 6. Open Decisions to Resolve Before/During Designer Contracts

1. `DIR` granularity: `Mutex` vs sleep-capable `RwMutex` (reader concurrency).
2. B/C-1 upper/workdir claim carrier: inode-owned `Extension` runtime lease
   vs persistent xattr reservation. Linux evidence (`I_OVL_INUSE`) favors the
   runtime lease unless cross-reboot/cross-mount persistence is required.
3. VFS inlet guard (`lookup_child()` children-cache `RwMutex` into lookup) —
   owned by a future VFS lookup reservation; Designer records it as a handoff
   obligation.

## 7. Pass Slicing Decisions

- None. No Creator/Checker/Reviewer passes and no pass slices exist. The
  57/24 Stage-D classification is untouched.

## 8. Explicit Agent-Level Decisions

- Closed the lock-topology-convergence phase without a convergence list or
  artifact repair (user: already reviewed during the design-document phase).
- Marked `UPPER` (and possibly `WL`) as `可能清理`, deferred to
  Designer/Creator stages.
- Cleaned old state: superseded the 2026-07-24 handoff; made this file the
  single live handoff; mirrored the board.
- **Designer wave 1 dispatched (2026-08-01):** subagent executed
  `subagent-tasks/mount_resource_policy/meso_01_mount_resource_policy_designer_dispatch.md`;
  artifacts written to
  `.agents/components/mount_resource_policy/meso_01_mount_resource_policy_designer_{spec,validation}.md`.
  Claim decision: 方案 A (inode `Extension` runtime lease) primary, 方案 B
  (xattr reservation) rejected for this wave (grounded in verified VFS
  evidence; fail-closed capability probe). Main-agent review: ACCEPT-with-notes
  pending user review — (1) rename `upper_dir`/`work_dir` to `upperdir`/
  `workdir` for consistency; (2) correct the `FsCreationCtx::task_ctx` claim
  (private field, no getter — record VFS dependency or use `PosixThread`
  credential route for P1-19); (3) resolve `Arc<OverlayLayerStack>` vs
  by-value sharing between `RootProjectionInputs` and `OverlayMountRuntime`;
  (4) the root-carrier seam awaits the `visibility_projection_identity`
  Designer. Consequence for user: the instance-stability probe excludes
  virtiofs/NFS-class upper backends until the identity contract lands
  (EOPNOTSUPP fail-closed).
- **Designer revision 02 dispatched (2026-08-01):** user-confirmed
  decisions — 64-bit unified UUID/claim-token entity with `P2-11` promoted to
  `需要实现` (scope 57/24; meso scope 9/6); `MountBuilder` removed in favor of
  a single `OverlayFs::new` constructor; P1-20 kept minimal-advisory with the
  VFS freeze-API gap recorded; `UpperWorkdirClaim` name retained; visibility
  ceiling for overlayfs code set at `pub(super)`/`pub(in
  crate::fs::fs_impls::overlayfs)` with `pub(crate)` requiring proof; renames
  `OverlayFs`/`MountPolicy`/`MountLifecycle`/`CredentialSource::Creator`;
  `RootProjectionInputs` removed (root carrier consumes `Arc<OverlayFs>`);
  helpers converted to methods. Board/map/ledger mirrored by the main agent.
- **meso 02 Designer contract ACCEPTED (2026-08-01, user review):**
  `visibility_projection_identity` is `Specified` (11/2, validation deferred);
  final carrier model + intermediate hygiene + LayerLookup naming all settled
  (revisions 04-06). Blueprint mirrored.
- **WORK PAUSED (2026-08-01, user direction):** meso 03
  (`merged_directory_index`) is NOT to be dispatched. The next scheduled
  action is the meso 03 Designer wave, but no further Designer/Creator/
  Checker work may start until the user explicitly resumes.
- **meso 01 Designer contract ACCEPTED (2026-08-01, user review):** the
  `mount_resource_policy` spec + validation contract (with the deferral
  clause) are accepted; the mount Meso is `Specified`. Next: dispatch the
  `visibility_projection_identity` Designer wave (meso 02, scope 11/2), which
  must freeze the `OverlayRootInode::new(Arc<OverlayFs>)` seam and resolve the
  DIR Mutex/RwMutex decision.
- **meso 01 validation deferred (2026-08-01, user-confirmed):** with only
  meso 01 implemented, no validation-contract case can pass (root-carrier seam
  step 10 blocks all mount-success paths; EROFS path needs sibling Mesos).
  Creator-synced runtime Checker for meso 01 = compile preflight only; the
  mount-group xfstests rows move to a meso-integration obligation after
  `visibility_projection_identity` + minimal read path. Validation contract
  amended via Designer revision 03; ledger note added.
- **meso 02 revision 04 dispatched (2026-08-01):** user-confirmed final
  carrier model — deleted `OverlayRootInode`/`DirDomain`/`OverlayDirFacts`/
  `OpaqueStatus`/`CachedBinding`/`BindingResult`/`BindingEntry`/
  `OverlayProjection`; unified `Binding` algebra with
  `PositiveBinding { kind, inode: Arc<OverlayInode> }` and
  `HiddenEvidence` (strong pin); `OverlayInode { fs: Weak<OverlayFs>,
  key: RealObjectKey, facts: Mutex<OverlayObjectFacts>, dir_transaction_lock:
  Option<Mutex<()>>, object_id, extension }`; `OverlayFs` gains `bindings`/
  `inodes`/`identity` (cross-meso owner rule); no marker cache (opaque
  re-observed per lookup under DIR); frozen seam
  `OverlayInode::new_root(fs) -> Arc<dyn Inode>` supersedes the provisional
  name in meso-01 step 10; meso-02 map Macro-Owner updated to `OverlayFs`.
- **Cross-meso owner-extension rule (2026-08-01, user-confirmed):** when a
  new module/field/method must be added to a generic runtime owner that first
  appeared in an earlier Meso (e.g., `OverlayFs`, `OverlayInode`), the
  consuming Meso describes the addition in its own Designer spec and records
  it as a consumption contract; NO bounded revision of the owner's first
  Meso is required. Applied to meso 02: `OverlayFs::overlay_dev_id()` and
  `MountPolicy::xino_mode()`/`XinoMode` are described in the meso-02 spec
  (§3.5) instead of a meso-01 revision; the Creator implements the additions
  when the consuming Meso's pass lands.
- **Protocol amendment (2026-08-01): design-root workflow.** The workspace
  adopted a design-document-driven workflow (`PROTOCOL.md` §0.5): the
  `designdoc/` drafts are the authoritative design root, and the Designer's
  job is to map them into a concrete meso-level Rust code form — module
  layout, structs, enums, carrier types, and helper signatures — per the
  Asterinas coding guidelines (`priors/ASTERINAS_CODE_QUALITY_PRIORS.md` +
  `book/src/to-contribute/coding-guidelines/`). All clauses that previously
  forbade Designer signature design are amended in `PROTOCOL.md`,
  `protocol/DESIGNER.md`, the designer-spec template, the workspace
  `README.md`, and the `ovfs-subagent` skill. Coding guidelines availability
  is confirmed.

## 9. Next Actions for the Next Thread (CRITICAL)

1. **[DONE 2026-08-01]** The meso 01 (`mount_resource_policy`, 9/6) and meso 02
   (`visibility_projection_identity`, 11/2) Designer waves are accepted; both
   are `Specified` with validation deferred to the integration gate. **WORK IS
   PAUSED at the user's direction: do NOT dispatch any Designer/Creator/
   Checker work until the user explicitly resumes.**
2. **On resume:** dispatch the meso 03 Designer wave for
   `merged_directory_index` (scope 4/1):
   - **Include the §9A distilled design-principles checklist in the packet**
     and review the Designer output against it (naming suffixes, wrapper/
     duplicate types, intermediates, visibility, VFS-claim truthfulness,
     testability analysis).
   - Inputs: accepted architect map
     `.agents/components/architect-reframed-topology/meso_03_merged_directory_index_architecture.md`,
     the Macro topology, design drafts
     `stageBCdraft/BC-3-merged-directory-readdir.md` (+ BC-2/BC-8), the
     accepted meso 01/02 contracts (published carriers: `OverlayFs` fields,
     `Binding` algebra, `OverlayInode`/`OverlayObjectFacts`, `DirDomain`
     payload-less DIR lock), the 57/24 scope labels, the `UPPER`/`WL`
     `可能清理` marker, and the open decisions.
   - Required artifacts per template (`designer_spec` + `designer_validation`,
     the latter with the testability/deferral analysis per the meso 01/02
     precedent).
3. Continue the remaining Meso-components in dependency order:
   `copyup_authority_file_views` -> `metadata_security_xattr_policy` ->
   `namespace_mutation_whiteout` -> `persistent_association_export`.
4. Only after Designer acceptance: record pass slices in `PASS_SLICING.md` and
   dispatch synchronized Creator/Checker loops per protocol gates (runtime
   rows deferred per each validation contract's deferral clause).

## 9A. Distilled Design Principles for Future Meso Designers (from meso 01/02 revisions, 2026-08-01)

*User direction: do NOT modify PROTOCOL.md for these principles. They are
review-derived rules distilled from the meso 01 (revisions 01-03) and meso 02
(revisions 04-06) repair cycles. The main agent MUST apply them when curating
future Designer packets and when reviewing Designer outputs.*

### 1. Naming
- Type names carry CONTENT, not generic role suffixes: no `Runtime`/`State`/
  `Entry`/`Result`/`Snapshot`/`Outcome` unless the semantics genuinely say so
  (`OverlayObjectFacts` holds facts; `MountLifecycle` holds lifecycle; a plain
  `MountPolicy` is the policy, not a snapshot of it).
- One concept = one word, one vocabulary family: unify the binding algebra
  (`Binding`/`PositiveBinding`/`NegativeBinding` used for cache AND results);
  align operation names with the micro-feature names
  (`lookup_in_layers`/`LayerLookup` <-> "Layer-ordered lookup", P0-08/P0-09);
  prefer Linux/reference terminology (`OverlayFs` <-> `ovl_fs`,
  `Creator` <-> `creator_cred`).
- Lock and field names explain WHY (`dir_transaction_lock`); lock-protected
  payloads are named by their content (`facts`, never `authority`).
- No word-order-inverted near-synonyms (`LayerObservations` vs
  `ObservedLayers` was rejected).
- Visibility: overlayfs code caps at `pub(super)` (visible to the whole
  `overlayfs` module) or `pub(in crate::fs::fs_impls::overlayfs)`; `pub(crate)`
  requires written proof (expected absent); helpers become methods/associated
  functions on their natural owners.

### 2. Structure
- No wrapper types that merely wrap an existing carrier (drop
  `OverlayRootInode`; the root is the `OverlayFs::root_inode` field).
- No field-for-field duplicate types (drop `PositiveObservation`; reuse
  `OverlayObjectFacts`).
- No near-identical multi-form types for one concept (drop
  `CachedBinding`/`BindingResult`/`BindingEntry`; one `Binding` algebra).
- Generic runtime owners accumulate extensions in the CONSUMING meso's spec
  (cross-meso owner rule); the owner's first meso is not revised.
- A mutex-protected struct contains ONLY what that mutex protects (claims left
  `MountState`; `MountLifecycle` holds phase only); pure serialization locks
  carry no payload (`dir_transaction_lock: Option<Mutex<()>>`).
- Single-valued object state lives on the inode (`facts`); per-name views live
  in the cache (`Binding`); a logical object has exactly one current
  authority, so the inode protects one state, not a binding series.
- Prefer simple constructors with RAII reverse-order rollback over Builder
  ceremony; keep helpers small by caller-side projection (`lookup_binding`
  does `project_inode` + assembly; `lookup_in_layers` stays a pure scan).
- Unify genuinely related identity entities when the design docs support it
  (claim token + UUID = one 64-bit `OverlayUuid`); keep deferred features as
  insertion points, do not pre-bake their fields (fsid stays only because
  P2-11 was promoted).

### 3. Intermediate-carrier hygiene
- Pure temporaries are locals, not named types; prefer streaming/merged
  passes over raw-materialization containers (`lookup_in_layers` replaced
  `LayerObservations`).
- Surviving intermediates must be: module-private, payloads reuse final types
  (no field duplication), listed in the complexity baseline with a one-line
  justification.
- (The protocol-level rule lives in `protocol/DESIGNER.md` §4; these are its
  operational consequences.)

### 4. Facts and verification
- Every cited VFS/OSTD interface must be verified read-only before freezing
  (the `FsCreationCtx::task_ctx` private-field lesson); unverifiable claims
  become recorded dependencies or alternate routes, never invented interfaces.
- VFS seam names signal their consumer (`OverlayInuseSlot`,
  `overlay_inuse_slot()`).
- Features present in the micro inventory but absent from the design docs must
  be surfaced as scope questions (P1-20), not silently given invented carriers.
- Heuristics carry explicit limitation notes (instance-stability probe).

### 5. Validation
- Testability analysis first: rows passable with the current meso set are
  early-PASS candidates; the rest defer to the meso-integration gate; the
  deferral clause lives in the validation contract (meso 01/02 precedent).

### MAIN-AGENT REMINDER
When the user resumes and the meso 03 (`merged_directory_index`) Designer wave
is dispatched: copy §9A into the dispatch packet as a checklist, and review
the Designer output against it. Do NOT modify PROTOCOL.md for these principles
(user direction); they live in this handoff only.


## 10. Live File Discipline

- **This file is the live handoff for:** 2026-08-01 State Cleanup & Designer
  Prep tenure (from 2026-08-01 onward).
- **Update rule:** Update this same file in place as Designer waves are
  dispatched and accepted.
- **Supersedes / Replaces:**
  `20260724-p0-p1-design-tracking_main_agent_handoff.md` (marked superseded).


## 11. Design Atlas — 七 Meso 设计总览（2026-08-02 通览）

*目标：让人能快速看懂整个 overlayfs 重构的设计。基础波次 6 个 Meso 有代码（01-06，全部 Specified）；meso 07 为 deferred-only（0/4）。*

### 11.1 模块树（Module Tree）

```text
kernel/src/fs/fs_impls/overlayfs/
├── mod.rs            // crate 根：注册 OverlayFsType(meso-01)；共享 AccessType{ReadOnly,Mutating}(meso-05)
├── fs.rs             // LEGACY，冻结；非设计源
├── mount/            // MESO-01 挂载资源与策略（被所有 Meso 消费）
│   ├── mod.rs        //   子模块声明；OverlayFsType(FsType impl)；meso 边界 re-export
│   ├── options.rs    //   OverlayMountOptions 解析（P0-01；is_default_permissions 等）
│   ├── layers.rs     //   OverlayLayerStack/OverlayLayer（不可变层栈；fsid/container_dev_id/root pin）
│   ├── claims.rs     //   OverlayUuid/InodeClaimGuard/UpperWorkdirClaim（IU 域：upper/workdir 独占 + 统一 64 位身份）
│   ├── policy.rs     //   MountPolicy（不可变策略）/CreatorCredentialPolicy/UpperFilesystemCapabilities/WriteAccessAccounting
│   ├── superblock.rs //   OverlayFs + FileSystem impl + MountLifecycle（MOUNT 域）
│   └── build.rs      //   OverlayFs::new 单构造器：有序步骤 + RAII 逆序回滚
├── projection/       // MESO-02 可见投影与身份（lookup/identity/ino 的唯一权威）
│   ├── mod.rs        //   OverlayFs 扩展（bindings/inodes/identity）；lookup_binding/project_inode 编排
│   ├── inode.rs      //   OverlayInode + OverlayObjectFacts + Inode impl + new_root（root carrier seam）
│   ├── entry.rs      //   RealObject + 层序观察/可见性归约（LayerLookup/lookup_in_layers）
│   ├── binding_cache.rs // Binding 代数 + BindingCache（(parent,name)->binding 首源）
│   ├── inode_cache.rs   // RealObjectKey + InodeCache（hardlink 身份共享）
│   ├── identity.rs      // IdentityPolicy：object_id、P0-12 dev/ino 矩阵、P2-01 xino、layer_devs
│   └── lower_id.rs      // P1-07：LowerIdRecord + store_lower_id/read_lower_id + project_object_id_from_lower_id
├── readdir_index.rs  // MESO-03 单文件扁平模块：ReaddirIndex（可见目录序列 + cookie/tombstone）
├── copyup/           // MESO-04 copy-up 权威与文件视图（委托上收 OverlayInode）
│   ├── mod.rs        //   Inode/FileOps 委托（read_at/write_at/.../open 副作用+None）
│   ├── coordination.rs // CopyUpTransition（CUL 载荷：publication_parent/name/phase）
│   ├── trigger.rs    //   ensure_upper_authority（CUL 内祖先作用域走查）
│   ├── promote.rs    //   promote/promote_directory/transfer_metadata/copy_eligible_xattrs/publish_upper_authority
│   └── workdir.rs    //   workdir temp 生命周期（generate/create/cleanup；workdir_temp_serial）
├── metadata_security/ // MESO-05 权限/元数据/xattr 策略
│   ├── mod.rs        //   OverlayInode 扩展；OverlayFs::xattr_policy accessor；delegate_to_real
│   ├── permission.rs //   两步权限管线：check_permission(access,perm) + check_local_permission/check_real_permission
│   ├── metadata.rs   //   set_mode/set_owner/set_group/set_atime/mtime/ctime
│   └── xattr.rs      //   OverlayXattrPolicy（classify/is_private/filter_private_names）+ XattrClass
└── dir/              // MESO-06 namespace mutation
    ├── mod.rs        //   create/mknod/write_link/link/unlink/rmdir/rename 入口；lock_dir_transaction/
    │                 //   lock_parent_dir_transactions；upper_parent；invalidate_stale_cache
    ├── create.rs     //   create_object(P1-23 分发器)/create_upper_only/create_over_whiteout
    ├── remove.rs     //   remove_target/visible_child_count
    ├── link.rs       //   link_source/link_over_whiteout
    ├── rename.rs     //   cross_device_gate/rename_upper/publish_rename
    └── whiteout.rs   //   WhiteoutCache(WL 域)/WhiteoutHandle/WhiteoutRepresentation/publish_whiteout
（meso-07：0/4 deferred-only；未来 index/export/nlink/离线检测插入点；index = 独立小模块、几乎完全是 copy-up 子模块）
```

### 11.2 Rust 代码结构树 + 设计目的（关键类型）

**mount/（meso-01）**
- `OverlayFs` — 挂载主体：policy/claims/root_inode/生命周期；所有 Meso 的载体宿主（各 Meso 按 cross-meso 规则扩展字段：facts 域、xattr_policy、whiteout_cache、workdir_temp_serial 等）
- `MountPolicy` — 不可变策略快照：is_effective_read_only(P0-18)/credential_policy(P1-19)/upper_capabilities(P0-02 探针)/uuid(P2-11)/is_default_permissions(P1-18 消费)
- `CreatorCredentialPolicy` — 挂载者凭证快照 + `with_creator_credentials_fn` 作用域覆盖（P1-19，底层凭证切换是记录依赖）
- `UpperFilesystemCapabilities` — 底层能力探针：can_store_private_xattr / can_mknod_char / can_report_directory_type
- `UpperWorkdirClaim` — IU 域：upper/workdir 独占 claim + 统一 64 位身份（P1-35/P2-11）
- `MountLifecycle` — MOUNT 域：Ready/ShuttingDown 过渡；teardown 等 pinned 消费者排空
- `OverlayLayerStack/OverlayLayer` — 不可变层栈（fsid/container_dev_id/root pin）

**projection/（meso-02）**
- `OverlayInode` — 逻辑 inode 载体：facts(Mutex<OverlayObjectFacts>, INODE 域) + dir_transaction_lock(DIR 域, payload-less) + copyup_transition(CUL, meso-04 扩展) + readdir_index(INODE, meso-03 扩展)；`Weak<OverlayFs>` 无环
- `OverlayObjectFacts` — 当前权威：upper/lowers RealObject 强 pin + kind（替换式更新，不原地改）
- `RealObject` — 真实对象（layer_index/real_inode/fsid/container_dev_id；强 pin 保活）
- `Binding`/`PositiveBinding`/`NegativeBinding`/`HiddenEvidence` — 统一 binding 代数（lookup 结果 = cache 项，单一类型）
- `BindingCache` — mount-wide `(parent,name) -> binding`；lookup 首源（RwMutex upread/upgrade）
- `InodeCache`/`RealObjectKey` — `(fsid,real_ino) -> Weak<OverlayInode>`；hardlink 共享身份
- `IdentityPolicy` — `object_id` 唯一身份源；dev/ino 矩阵（P0-12）+ xino 掩码（P2-01）+ layer_devs（policy 输入）
- `LowerIdRecord` — P1-07 持久 lower 身份（fsid+real_ino，无 runtime owner）；store/read seam + project_object_id_from_lower_id

**readdir_index.rs（meso-03）**
- `ReaddirIndex` — 每目录可见序列索引（INODE 域）：entries(Vec)/validity/next_cookie/tombstone_count
- `ReaddirIndexEntry` — `Visible{name,cookie,inode,type_}` | `Tombstone{name,cookie,Weak inode}`（墓碑不强 pin）
- `ReaddirCookie` — 单调永不复用 continuation cookie（`.`/`..` 保留 1/2；procfs-slot 约定）
- `ReaddirIndexValidity` — Valid/NeedsRebuild 两态（BC-3 §30 惰性重建；无 version）
- seams：insert_visible/update_visible/remove_visible/invalidate_readdir_index/compact_tombstones（tombstone_count >= live_count 时压缩）

**copyup/（meso-04）**
- `CopyUpTransition` — CUL 载荷：publication_parent/name/phase（发布坐标 + ReconcilePending；parent 可能 lower-backed）
- `CopyUpPhase` — Idle/ReconcilePending（无「copy-up completed」历史标记）
- 入口：ensure_upper_authority（无参；CUL 内祖先走查决定作用域）/ promote（按对象种类内部分发）/ publish_upper_authority（原子发布 + store_lower_id + object_id 保持 lower-derived）/ workdir temp 三件套

**metadata_security/（meso-05）**
- `OverlayXattrPolicy` — 私有/公开/转义分类（`is_private` 取代 meso-04 拷贝期谓词；无字段，stateless）
- `XattrClass` — Public/Private/Escaped/Reserved（无载荷）
- `AccessType`（共享于 crate 根）— ReadOnly/Mutating（no-bool-args）
- 管线：check_permission(access,perm)（公共入口 = check_local_permission + 除非 default_permissions 则 check_real_permission）+ delegate_to_real

**dir/（meso-06）**
- `WhiteoutCache` — WL 域载荷：cached(单槽)/can_share_by_link；take/store/disable_sharing（短临界区，无 BIO）
- `WhiteoutHandle` — workdir staging whiteout（inode 强 pin + workdir_name）
- `WhiteoutRepresentation` — CharDevice/Xattr（meso-01 能力派生，无运行时探针）
- 入口：create_object 分发器/remove_target/cross_device_gate/rename_upper/publish_rename/invalidate_stale_cache

### 11.3 逻辑承载点注释（该实现什么逻辑）

| 承载点 | 该实现的逻辑 |
| :--- | :--- |
| `mount/build.rs::OverlayFs::new` | P0-01/02/03/05/18/P1-19/20/P2-11：解析→层解析→claim→探针→workdir→身份→root carrier；失败 RAII 逆序回滚 |
| `projection/inode.rs::OverlayInode::lookup` | P0-08/09/10/11：DIR 内 `lookup_binding` → 层序可见性归约 → `project_inode` → 发布 binding（永不用陈旧 VFS dentry） |
| `projection/identity.rs::project_object_id(+from_lower_id)` | P0-12/P2-01/P1-07：dev/ino 矩阵 + xino 掩码 + 跨 copy-up st_ino 稳定（读 LowerIdRecord） |
| `projection/lower_id.rs::{store_lower_id, read_lower_id}` | P1-07：编码+持久 `trusted.overlay.origin`（meso-04 发布调用，先于 facts 替换）；读侧喂 object_id |
| `readdir_index.rs::readdir_at` | P0-13/14/15：合并/非合并 readdir + cookie 单调 + 跳墓碑 + `d_ino=object_id().ino`（procfs-slot delta 语义） |
| `readdir_index.rs::invalidate_readdir_index` 等 seams | P1-31：index Valid 时细粒度维护，否则 NeedsRebuild；tombstone 阈值压缩 |
| `copyup/trigger.rs::ensure_upper_authority` | P1-01/02/03：写意图提升；CUL 内 facts+坐标链祖先走查决定「几层/几支」；EROFS 门在入口 |
| `copyup/promote.rs::publish_upper_authority` | P1-04/05/06/P1-07：workdir temp + 原子 rename 发布 → `store_lower_id` → facts 替换且 `object_id` 保持 lower-derived |
| `copyup/workdir.rs` | P1-34：workdir temp 唯一命名（`#{name}#{parent_ino}#{serial}`）+ 有界 EEXIST 重试 + cleanup（P3-09 插入点） |
| `metadata_security/permission.rs::check_permission` | P1-18：本地检查（锁外）→ 除非 default_permissions → real 检查（creator-creds）；不缓存判定 |
| `metadata_security/metadata.rs` | P1-16/17：准入后经 `delegate_to_real` 转发 upper real；atime/EROFS 分歧保留 |
| `metadata_security/xattr.rs::OverlayXattrPolicy` | P1-33：私有名分类/过滤/拒绝（ENODATA/EPERM）+ 公开操作委托；分类先于副作用 |
| `dir/create.rs::create_object` | P1-23：从新鲜 Binding 分发 upper-only / over-whiteout / opaque（Absent/HiddenByWhiteout/HiddenByOpaque/Positive） |
| `dir/mod.rs` 的 create/link/unlink/rmdir/rename | P1-21..P1-30：物理 upper 操作（DIR 内）+ **DIR 释放前语义发布**（BindingCache + readdir_index seams）；失败 `invalidate_stale_cache` |
| `dir/whiteout.rs::publish_whiteout` | P1-25/36：WL 槽协议（take/store/disable_sharing）+ link 共享 / rename-over 发布 |
| `dir/mod.rs::invalidate_stale_cache` | Case-13 保守对账：物理成功+发布失败 → 每 (parent,name)：`BindingCache::invalidate` + `invalidate_readdir_index` |

### 11.4 跨 meso 一致性检查（2026-08-02）

**已修复（本 session 各 revision）**
1. P1-07/lower-id 归属：meso-07 → meso-02（无 runtime owner；`store_lower_id` 签名 meso-02/04 逐字一致）；meso-07 0/4 deferred。
2. st_ino 跨 copy-up 稳定：meso-04 不再 re-project from upper；`object_id` 保持 lower-derived（IdentityPolicy 读 LowerIdRecord）——与 meso-02 authority-continuity 不变量对齐。
3. `read_only_gate` 单一化：meso-06 删自声明，统一走 meso-05 `check_permission(Mutating)`（其 stage A 含 EROFS 门）。
4. naming 收敛：`mutation/`→`dir/`、`authority/`→`copyup/`、`association/`→(迁走)、`conservative_invalidate`→`invalidate_stale_cache`、`take_cached`→`take`、`OverlayOriginFh`→`LowerIdRecord`（经迁址）等。
5. WhiteoutRepresentation 能力派生：meso-01 发布 `can_mknod_char` + whiteout 挂载门（fail-closed）。
6. `AccessType` 提升共享；`d_ino` = `object_id` 单源（stat/readdir/copy-up 稳定全部收敛到 object_id）。

**确认一致**：锁序 `DIR → CUL → INODE → WL → UPPER`（各 Meso 消费自己的子集，无新锁级）。**完整锁偏序（含缓存锁，2026-08-02 跨 meso 审计后记录）：** mount-wide `OverlayFs::bindings`/`inodes`（`RwMutex` upread/upgrade）是「FS 大锁」但为 **DIR 嵌套内部锁（非拓扑级）**——只在 DIR 下（或单独）获取、两把顺序使用互不嵌套、**绝不与 CUL/INODE/WL/UPPER 共存**。审计站点：meso-02 lookup（DIR→bindings.get→释放→inodes.get_or_create→释放→bindings.insert）、meso-03 readdir scan（index 锁外做 lookup_binding）、meso-04 `record_copyup_transition`（try_lock CUL，永不阻塞，无死锁；`publish_upper_authority` 不碰缓存）、meso-06 mutation 发布（bindings + readdir_index 顺序执行）。因此五锁 DAG 对操作路径完备，无需新增「FS 大锁」级；Linux 对照：overlayfs 无 mount-wide 操作锁，缓存走 VFS 全局 inode hash 锁（同样不在 overlay 锁序内）。可选后续：Architect map 加一行显式注记（缓存锁 = DIR 嵌套内部锁，非拓扑级）——Architect-routed。锁形态确认（2026-08-02，对照 `ostd/src/sync/`：`Mutex`/`RwMutex` 均 sleep-capable，`RwLock` 为 spin 系不可用于 BIO 路径）：DIR = `dir_transaction_lock: Option<Mutex<()>>`（sleep-capable `Mutex`，payload-less；meso-02 §3.0 明确非 RwMutex）；CUL = `copyup_transition: Mutex<Option<CopyUpTransition>>`（`Mutex`，copy-up 可 BIO）；INODE = `facts: Mutex<OverlayObjectFacts>` + meso-03 `readdir_index: Option<Mutex<ReaddirIndex>>`（均 `Mutex`；mount-wide `BindingCache`/`InodeCache` 是唯一的 `RwMutex`(upread/upgrade) 使用点，不属于五把操作路径锁）；WL = `whiteout_cache: Mutex<WhiteoutCache>`（`Mutex`，临界区绝无 BIO；Mutex-vs-RwMutex 仍 deferred）；**UPPER = 无 Overlay 自有锁**（未冻结的 `可能清理` 候选，real upper/workdir 操作走底层 fs 自身锁 + 调用方 DIR）。另：MOUNT(Level 1) = `lifecycle: Mutex<MountLifecycle>`；IU = out-of-band `AtomicBool`/CAS（非 mutex）。meso-06 消费 meso-03 的 index seams 与 meso-05 准入；meso-04 消费 meso-02 的 seam（store_lower_id/facts_snapshot/select_real_inode）；meso-07 未来 index 与 lower-id 概念分离。

**待办 handoff（非矛盾，已记录）**
1. meso-06 §4.1 消费的 meso-03 `readdir_index_insert/remove` 决策 seams：由 meso-03 Creator 按 consumption-seam 模式实现（meso-03 spec 不另修订）。
2. meso-02 `xino_mode` 依赖 meso-01 发布（`MountPolicy::xino_mode()`）：默认 Auto 已可用；正式发布待 meso-01 小修订（与 default_permissions 同类）。
3. WL Mutex/RwMutex：deferred（所有 meso 结束后定）。
4. meso-02 `nlink==1` 门（P2-07 interplay）：deferred 消费决策。
5. meso-02 `layer_devs` 字段：已记录（policy 输入，非运行时状态）。
