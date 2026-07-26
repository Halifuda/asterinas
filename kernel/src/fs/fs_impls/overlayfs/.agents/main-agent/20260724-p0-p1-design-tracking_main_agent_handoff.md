<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff - 2026-07-24 P0/P1 Design Tracking

**Date / Time:** 2026-07-30
**Status:** `Open; Architect topology reset complete; Designer not started`

## 1. Global State Pointer

- **Current Active Wave / Pass:** None; the fresh Architect topology reset is
  complete and accepted, with no implementation pass active.
- **Blueprint Updates Made:** Yes. `SYSTEM_BLUEPRINT.md` and
  `PASS_SLICING.md` now supersede the old 13-Meso/Designer state and register
  the new seven-Meso topology.
- **Accepted baseline:** The fresh Macro plus seven Meso architecture maps own
  all 81 formal Micro IDs. The old Architect topology and its Designer wave
  are discarded; no current Designer contract exists.
- **User-directed change:** The next week is reserved for a read-only code
  inspection and design-document study. The previous implementation
  recommendation for `mount_options / P0-01` is suspended for this tenure.
- **Design-document goal:** Produce a human-readable design document for a
  basic Overlay FS, covering all P0 and P1 behavior and only the closely
  related P2/P3 behavior needed to make that foundation coherent.
- **Strategic context (not document content):** The broader motivation is to
  make sandbox-oriented tooling easier to integrate into Asterinas. This
  motivation guides prioritization but must not appear as a requirement or
  product strategy in the design document.

## 2. Design Scope

- **Confirmed baseline scope:** All P0 and P1 Micro IDs, currently 55 items.
- **Adjacent scope policy:** Include a small number of P2/P3 items only when
  the top-down model or a core P0/P1 workflow shows that omitting them would
  leave the basic Overlay FS design incomplete. Inclusion requires an
  interactive decision with the user.
- **Initial review queue, not accepted scope:**
  - `P2-02` `redirect_dir`, coupled to rename and `EXDEV` behavior.
  - `P2-05` POSIX ACL, coupled to permission and metadata updates.
  - `P2-07` nlink preservation, coupled to hardlink and copy-up behavior.
  - `P2-12` fsync mode, coupled to copy-up publication and file fsync.
  - `P2-13`/`P2-14` xattr namespace and nested escaping, coupled to P1 xattrs.
  - `P2-15` layer casefold support, coupled to lookup and revalidation.
  - `P3-08` trap inodes and `P3-09` workdir cleanup, coupled to dentry
    lifecycle and mount/workdir correctness.
- The final P2/P3 set is decided after the overall model and core workflows
  are understood, not by reading the feature list in numeric order.

The design milestone is complete only when the document explains the basic
Overlay FS model, ownership and authority, lifecycle, global invariants,
core workflows, selected adjacent features, and the traceability/validation
evidence for every selected Micro ID.

## 3. Interactive Working Method

1. The main agent presents one bounded topic at a time using the staged priors,
   accepted Architect map, and accepted Designer contracts.
2. The user explains, challenges, or confirms the mental model and unresolved
   decisions. The main agent records decisions only after confirmation.
3. Each confirmed topic produces three outputs: a concise model, explicit
   invariants/lock hazards, and its traceability/validation references.
4. Cross-topic conflicts are surfaced immediately and resolved before the
   topic is considered design-complete.
5. The main agent maintains a running list of open questions and periodically
   gives a compact design-meeting rehearsal summary.

The unit of progress is a confirmed design topic, not a calendar day. The user
may reorder, narrow, or defer any topic without creating a new handoff file.

## 4. Top-Down Design Sequence

- **Stage A: Document goal and overall semantic model.** Define the basic
  Overlay FS target, its user-visible promise, the upper/lower/layer model,
  the visible namespace, and the relationship between real and overlay
  objects. Do not begin with a Micro-feature matrix.
- **Stage B/C: Joint ownership, lifecycle, invariant, and concurrency model.**
  Design these together by semantic module rather than completing all ownership
  work before concurrency work. Establish who owns each state, who may publish
  changes, what lifetime rules apply, and how blocking/BIO boundaries, re-entry,
  lock topology, permission invariants, copy-up publication, rollback, and
  persistence obligations constrain that state.
- **Stage B/C module order:** mount/layer/upper-workdir; projection/identity/
  lookup; merged directory/readdir; copy-up/file I/O/page cache;
  metadata/permission/xattr; directory mutation/whiteout; optional advanced
  identity/export/data features; then cross-module reconciliation.
- **Stage D: Scope decision.** Confirm all P0/P1 as the baseline, then use the
  model and workflows to decide which closely related P2/P3 items are needed.
  Record inclusion rationale and explicit exclusions.
- **Stage E: Core end-to-end workflows.** Walk through mount, lookup/stat/
  readdir, open/read, write-triggered copy-up, metadata/permission, create,
  whiteout/unlink/rmdir, link, rename/`EXDEV`, fsync, and cleanup.
- **Stage F: Meso responsibility decomposition and design closure.** Map each
  workflow to its Meso-components, define VFS handoffs and ordinary underlying
  lookup boundaries, resolve cross-meso ownership/lock questions, and retain
  traceability/evidence references from the accepted Architect/Designer
  artifacts. This is the final design stage; it does not freeze Rust signatures
  or schedule implementation passes.

## 5. Required Design Outputs

- One comprehensive basic Overlay FS design document or synthesis index.
- A complete selected-Micro traceability matrix.
- A cross-meso workflow and invariant/lock table.
- An xfstests-only external validation mapping, distinguishing direct,
  combined, not-run/unsupported, and no-upstream-coverage evidence.
- An open-decisions and risks register, including Linux/Asterinas divergences.

Any future Designer artifacts must remain Meso-scoped and retain their protocol
templates; this topology-reset wave intentionally creates none. The current
design synthesis and the accepted fresh Architect maps are the only design
inputs for the next scheduling decision.

## 6. Artifact Placement

- **Primary human-readable design document:**
  `.agents/designdoc/OVERLAYFS_BASIC_DESIGN.md`
  - Main top-down narrative for the design exchange.
  - Main-agent-owned and tracked with the overlayfs protocol files.
  - Contains the overall model, ownership/lifecycle, invariants, workflows,
    selected P2/P3 rationale, and links to supporting artifacts.
- **Stage A working draft:**
  `.agents/designdoc/stageAdraft.md`
  - Chinese-language consolidated draft for user review before synthesis into
    the primary design document.
  - Covers the Stage A goal, terminology, layer/visibility model,
    Overlay/underlying VFS boundary, upper writability, copy-up responsibility,
    and persistence/crash-consistency boundary.
  - Cross-checks the consolidated model against the local Linux overlayfs
    documentation and implementation; it is not a temporary patch record.
- **Stage B/C working draft:**
  `.agents/designdoc/stageBCdraft/README.md`
  - Chinese-language index and module ledger for the joint
    ownership/lifecycle and invariant/concurrency design.
  - Each stage is stored in its own file under the same directory:
    `BC-1-mount-layer-upper-workdir.md` through
    `BC-8-cross-module-reconciliation.md`.
  - The split keeps later interactive work scoped to one stage instead of
    loading the entire B/C draft into context.
- **Completeness appendix:**
  `.agents/designdoc/OVERLAYFS_BASIC_DESIGN_TRACEABILITY.md`
  - Selected Micro IDs, Meso ownership, spec sections, dependencies, and
    xfstests evidence classification.
  - Bottom-up and audit-oriented; it is not the primary reading order.
- **Meso-level source artifacts:**
  `.agents/components/architect-reframed-topology/`
  - The fresh Macro and seven Meso architecture maps are the only current
    topology artifacts. The superseded component directories and old Designer
    dispatch artifacts were deleted.
  - No Designer spec/validation artifact is generated in this reset wave.
  - Do not place the cross-meso synthesis document in a synthetic component
    directory or mix it with Creator/Checker artifacts.
- **Tracking and decisions:** The single live file under `.agents/main-agent/`
  remains the handoff and records process decisions, open questions, and next
  actions. It is not the design document.
- **Priors and protocol:** `.agents/priors/` and `.agents/protocol/` remain
  read-only source/reference locations for this design tenure.
- **Out of scope for placement:** No design document is placed beside the
  production Rust files, in `book/`, or under a ktest/xfstests harness.

## 7. Constraints

- No production Rust edits, Creator packets, implementation pass slicing, or
  runtime build/test execution during this design tenure.
- No ktest, filesystem-local test, memory-disk fixture, or other internal test
  lane may be created or implied. Validation discussion remains xfstests-only.
- Do not use the legacy `overlayfs/fs.rs` implementation as a design source;
  use the staged specification, reference summary, integration priors, and
  accepted Architect/Designer artifacts.
- Do not freeze new Rust signatures, private helper APIs, or implementation
  pass boundaries in the design discussion.

## 8. Explicit Agent-Level Decisions

- Supersede the 2026-07-22 implementation handoff for this tenure.
- Confirm all P0/P1 as the foundation scope.
- Select P2/P3 only after the top-down model and core workflows demonstrate
  their necessity or close coupling.
- Keep the sandbox integration motivation out of the design document.
- Use the top-down narrative as the primary acceptance shape and the
  bottom-up protocol artifacts as completeness/traceability support.
- Use user-confirmed conceptual checkpoints as the acceptance condition for
  design progress; do not infer understanding from document generation alone.
- Put the human-readable synthesis and its traceability appendix under
  `.agents/designdoc/` as specified above.

### Stage A Confirmed Semantic Model

- **Namespace vs. projection:** The visible namespace is the externally
  observable mapping from paths/names below the Overlay mount root to semantic
  visibility results. An Overlay projection is the internal in-memory carrier
  for one such result, including its selected upper object, ordered visible
  lower objects, merged-directory state, and visibility barriers. The two
  concepts must remain distinct in the design document.
- **Overlay/VFS boundary:** Overlay implements the semantic membrane between
  the VFS-facing operation and the underlying filesystems. It resolves
  visibility, identity, directory merging, whiteouts/opaque state, copy-up,
  mutation routing, and publication ordering; it then dispatches primitive
  operations to the selected underlying VFS objects. Underlying filesystems
  remain the owners of real data, metadata, and durable objects.
- **Writable configuration:** A mount is writable only when it explicitly
  configures a valid upper/workdir pair. An existing directory beneath a
  lower layer, or a potentially writable lower filesystem, does not make the
  Overlay writable. With no configured upper, all mutating operations are
  rejected before changing any lower object.
- **Persistence contract:** Overlay does not provide a general transaction or
  stronger crash-consistency guarantee than the upper filesystem. It may use
  temporary workdir state, atomic underlying operations, publication ordering,
  and forwarded fsync calls to preserve its live namespace semantics, but it
  does not promise multi-object rollback or a unique post-crash state beyond
  the guarantees of the underlying filesystem and selected sync behavior.
- **Stage A status:** Confirmed interactively on 2026-07-24. The basic model
  is accepted for progression to ownership and lifecycle; the detailed fsync
  mode decision remains deferred to the later adjacent-scope review. The
  consolidated Chinese review draft is at
  `.agents/designdoc/stageAdraft.md` and is awaiting user approval.

### Stage B/C Joint Design Decision

- **Decision:** Stage B and Stage C are jointly designed and are split into
  semantic modules. Each module must simultaneously specify ownership,
  lifecycle, invariants, concurrency, blocking/re-entry boundaries, and
  cross-module handoffs.
- **Design draft index:** `.agents/designdoc/stageBCdraft/README.md`
- **Module schedule:** B/C-1 through B/C-8 as listed in the index and linked
  stage files. All eight modules are complete after the final cross-module
  reconciliation; no module is considered complete from its existence in the
  outline alone.
- **Scope rule:** B/C-7 advanced features remain conditional on demonstrating
  their necessity or close coupling to the P0/P1 foundation. This decision does
  not start implementation, pass slicing, runtime validation, or test work.

### B/C-1 Detailed Discussion Checkpoint

- **Status:** The detailed discussion draft is in
  `.agents/designdoc/stageBCdraft/BC-1-mount-layer-upper-workdir.md`. The user has
  confirmed the RAII/Builder direction and the draft is sufficient to enter
  B/C-2; explicitly listed B/C-1 open decisions remain deferred and are not
  silently resolved by B/C-2.
- **Covered topics:** runtime state carriers, mount construction phases,
  upper/workdir semantics, shared upper/workdir exclusivity, Asterinas's
  `FileSystem::root_inode()` and `Mount::new()` publication boundary, failure
  rollback, blocking/re-entry rules, and handoffs to projection/copy-up/mutation.
- **Important open decisions:** semantics of workdir without upper; whether a
  configured upper must still claim workdir when the mount is forced read-only;
  and the exact shared runtime carrier for cross-mount upper/workdir claims.
- **Confirmed construction decision:** Mount construction uses a temporary
  Builder that owns layer references, credential snapshot, upper/workdir claim
  guards, workdir preparation state, candidate identity, and root inputs. A
  single commit publishes `OverlayMount`; before commit, ordinary runtime
  ownership cleanup is RAII-driven in reverse dependency order. Workdir cleanup
  and UUID xattr writes remain explicit operations because RAII does not provide
  fallible persistent rollback.
- **Review rule:** User has confirmed the B/C-1 draft.

### B/C-2 Completion Checkpoint

- **Status:** Completed after the interactive projection, identity, carrier,
  lookup-lock, publication, whiteout, cache, and absent-revalidation
  discussion. The accepted text is in
  `.agents/designdoc/stageBCdraft/BC-2-projection-identity-lookup.md`.
- **Boundary:** B/C-2 consumes B/C-1's published layer snapshot, mount
  lifetime, and resolved identity policy. It owns neither a second layer
  registry nor a second durable identity mapping.
- **Static content:** The draft separates mount snapshots, real path views,
  `OverlayEntry`/`OverlayInode` carriers, temporary lookup observations,
  semantic visibility results, and derived identity projections. It keeps
  namespace visibility authority, metadata authority, and future data
  authority distinct.
- **Dynamic workflow:** The draft specifies pin/plan, one Overlay parent `DIR`
  lookup cycle, underlying helper-managed real-parent locks, upper-first
  reduction, conditional lock-neutral handling only for proven re-entry or
  reverse-order cases, unpublished carrier construction, publication, identity
  derivation, and stale/negative-result revalidation.
- **Concurrency boundary:** Lookup enters through parent `DIR`; relevant
  `UPPER` access remains below it and same-level instances use the accepted
  `Arc::as_ptr()` order. B/C-2 does not acquire `CUL`, `INODE`, or `WL`, and
  does not hold unknown/reentrant underlying callbacks under Overlay locks.
- **Deferred details:** The exact Rust realization of VFS private payload and
  atomic positive/negative publication, identity continuity across copy-up,
  and the conditional xino/casefold/trap scope remain deferred to the later
  interface and advanced-feature decisions. They do not block the B/C-2
  semantic design.
- **Completion basis:** The static carrier split, authority boundaries,
  positive/negative binding algebra, inode-cache and binding-cache roles,
  one-`DIR` lookup workflow, whiteout/opaque invalidation, and
  `REVALIDATE_ABSENT` behavior are recorded and accepted for progression to
  B/C-3.

### B/C-2 Interactive Refinement - Lock Boundary (2026-07-26)

- **User question:** The draft's `lock-neutral` workflow appeared to release
  and reacquire locks repeatedly. The user supplied `/home/ayd/linux` as the
  reference source and asked how Linux handles lookup.
- **Linux evidence:** `fs/namei.c:1929-1938` acquires the Overlay parent
  `i_rwsem` once around `__lookup_slow()`; `fs/overlayfs/namei.c:1382-1420`
  performs the complete Overlay lookup and layer reduction in that interval.
  Per-layer `fs/overlayfs/namei.c:211-217` calls
  `lookup_one_unlocked()`, whose `fs/namei.c:3211-3241` contract manages the
  corresponding real filesystem parent's lock internally.
- **Design correction:** B/C-2's default workflow keeps one Overlay parent
  `DIR` across the full semantic lookup, including ordinary BIO/sleep. It may
  perform multiple short real-parent lock acquisitions, but it does not
  repeatedly release/reacquire Overlay `DIR`. `lock-neutral` is only a
  conditional callback boundary for proven synchronous Overlay re-entry or
  reverse lock ordering; it is not the default per-layer lookup mode.
- **Asterinas distinction:** The current children cache uses the
  sleep-capable `ostd::sync::RwMutex`, not the spin-based `ostd::sync::RwLock`.
  `Dentry::lookup_child()` nevertheless carries that generic VFS cache guard
  into `inode.lookup()`, which is a lock-order/re-entry hazard independent of
  whether the guard can sleep. A future VFS lookup reservation should release
  the cache guard before the filesystem callback and publish under the
  reservation afterward. This VFS cache boundary is a different owner from
  Overlay `DIR` and does not justify releasing/reacquiring Overlay `DIR`.
- **Artifact consistency note:** The accepted topology/designer artifacts
  still contain older broad wording that treats an insufficiently proven
  callback as lock-neutral by default, and one identity artifact says an
  inode carrier cannot outlive its overlay entry. These are now identified as
  wording requiring a bounded artifact repair before implementation; they are
  not silently rewritten during this interactive checkpoint.

### B/C-2 Interactive Refinement - Identity and Carrier Direction (2026-07-26)

- **No reverse name mapping:** `ID projection` produces the policy-qualified
  overlay identity from an already selected logical object. It does not provide
  `ID -> name`; hard links make that relation non-single-valued, and layer,
  parent, whiteout, opaque, and revalidation state make a bare inode number
  insufficient to describe namespace visibility.
- **Inode cache role:** An inode cache is required, but it is a carrier-reuse
  cache keyed by a mount/layer/real-object-qualified `OverlayObjectKey`, never
  by bare `st_ino`. Multiple named bindings may share one `OverlayInode`, while
  each `(parent, name)` binding keeps its own `OverlayEntry` projection.
- **ID-consuming APIs:** Ordinary path syscalls resolve paths or fds and do
  not use `st_ino`/`d_ino` to find a path. A future export/file-handle or
  filesystem-specific object-ID API must use a separate explicit object index
  and object authority contract; it cannot be implemented as `ino -> name`.
- **Lifetime direction:** `OverlayInode` strongly retains mount/layer state so
  a live inode cannot outlive its real references. The identity cache owns only
  weak/reclaimable carrier slots. After root publication, VFS owns the strong
  root carrier; mount state must not strongly own an inode that strongly owns
  the mount state.
- **Draft update:** The B/C-2 stage file contains the three-projection
  comparison, the identity/cache boundary, the corrected carrier graph, and
  the lookup lock rule. This checkpoint was later consolidated into the
  completed B/C-2 design; no implementation or pass slicing is authorized.

### B/C-2 Interactive Refinement - Publication Shape (2026-07-26)

- **Static carrier correction:** The draft now separates `EntryBinding` (root or
  `(parent, name)`), temporary `NameObservation`, published `NameView`,
  `OverlayEntry` private dentry state, `OverlayObjectState`,
  `OverlayDirectoryState`, and `OverlayInode`. A named
  `OverlayEntry` is a per-binding carrier; the inode is the logical-object
  identity/authority carrier; the shared object state is the sole mutable
  upper/lower publication authority.
- **Root graph correction:** Root publication uses the same inode/private-state
  pairing as named publication but has no parent/name. After publication, VFS
  root dentry/private state holds the strong root references; mount state keeps
  only an optional weak root handle and the inode cache keeps weak/reclaimable
  slots, avoiding a mount-to-root-to-mount strong cycle.
- **Projection direction:** Name/root observation produce `ObjectInput`; only
  a positive name observation enters ID projection. ID projection creates or
  reuses the logical-object inode carrier and never triggers another name
  lookup, directory scan, or parent `DIR` acquisition.
- **Publication boundary:** The conceptual VFS boundary is now an explicit
  `LookupPublication::Positive { inode, private_state }` or
  `LookupPublication::Negative { private_state }`, plus a one-time
  `RootPublication`. Negative state preserves whiteout/opaque barriers and
  version facts without creating an inode. This is a semantic requirement for
  Stage F, not a frozen current Rust signature.
- **Linux reference:** The local Linux source under `/home/ayd/linux` confirms
  the one-Overlay-parent-lock rule: `fs/namei.c:1929-1938` wraps one
  `__lookup_slow()` call with the parent lock, `fs/overlayfs/namei.c:1382-1420`
  performs the full overlay lookup in that interval, and
  `fs/overlayfs/namei.c:211-217` delegates per-layer real-parent locking to
  `lookup_one_unlocked()` (`fs/namei.c:3211-3241`).

### B/C-2 Interactive Refinement - Linux carrier fact correction (2026-07-26)

- **Source correction:** `/home/ayd/linux/fs/overlayfs/ovl_entry.h:153-188`
  shows that Linux associates `struct ovl_entry` with `ovl_inode->oe`; the
  `dentry->d_fsdata` access in `OVL_E_FLAGS()` is for dentry flags, not the
  complete lower-stack carrier. `ovl_lookup()` then initializes dentry
  revalidation state and returns through `d_splice_alias()`.
- **Design consequence:** B/C-2's `OverlayEntry` remains a semantic role name
  for the proposed Asterinas per-binding private snapshot. It must not be
  described as a direct Linux `struct ovl_entry` equivalent. Shared logical
  object/lower namespace authority remains in `OverlayObjectState` and
  `OverlayInode`; root and named VFS bindings may carry separate snapshots.
- **Current VFS gap:** Asterinas `Dentry` currently owns an `Arc<dyn Inode>` and
  name/parent/cache state but has no generic filesystem-private payload;
  `Inode::lookup()` returns only `Arc<dyn Inode>`, and
  `CachedDentry::Negative` has no payload. A future lookup reservation must
  therefore publish positive/negative targets and filesystem-private state as
  one VFS transaction. This is an explicit Stage F interface requirement, not
  a production-code change in this design tenure.
- **Static carrier refinement:** The draft now explicitly carries ordered
  `ObjectInput.lower_namespace`, makes `OverlayObjectState` the sole mutable
  upper/lower publication authority, and keeps the VFS dentry as the live
  `(parent, name)` owner while `OverlayEntry` remains a binding snapshot.
  The current recommended publication shape is a positive dentry with generic
  private state plus a negative cache variant that also carries generic
  private state; an explicit positive/negative `DentryTarget` remains only an
  alternative VFS realization.

### B/C-2 Interactive Refinement - Transactional publication (2026-07-26)

- **Lookup transaction:** The named-child path is now explicitly modeled as a
  VFS `(parent, name)` reservation followed by one Overlay parent `DIR`, then
  underlying observation, visibility reduction, ID projection, and reservation
  publication before releasing that `DIR`. This prevents the conceptual
  `project_named_child()` return boundary from accidentally moving VFS
  publication outside the directory consistency interval.
- **ID lock inlet:** “ID projection has no `DIR`” means it never acquires,
  releases, or recursively reacquires `DIR`; a name transaction may call it
  while already holding the parent `DIR`, because ID projection consumes only
  prepared `ObjectInput` and performs no underlying lookup.
- **Carrier shape:** The draft's VFS-like shape now mirrors the current
  Asterinas taxonomy (`Dentry` plus `DentryDirState`) while marking generic
  positive private state and negative-cache private state as a future VFS
  reservation/publication capability, not as current API.

### B/C-2 Completion Record (2026-07-26)

- **Result:** B/C-2 is complete as a semantic design module. The result does
  not freeze Rust types or change the current VFS API.
- **Root projection:** Mount commit projects B/C-1's pinned real root and
  identity policy into a root binding and root inode carrier before VFS root
  publication.
- **Name and ID projection:** Name projection reduces upper, whiteout, opaque,
  and lower observations into a positive or negative binding result. Only a
  positive result enters ID projection. ID projection creates or reuses an
  inode carrier from a policy-qualified object key and never supplies an
  `ID -> name` mapping.
- **Binding and inode caches:** The per-name binding cache retains projection
  and hidden-state evidence; the inode cache reuses logical-object carriers and
  is not keyed by a bare `st_ino`. Multiple bindings may share one inode.
- **VFS and revalidation:** Positive and negative visibility remain distinct;
  whiteout/opaque does not become a fabricated inode. Overlay directories use
  `REVALIDATE_ABSENT`, with cheap validation and lookup fallback when negative
  state cannot be proven current.
- **Concurrency and invalidation:** The normal lookup holds one Overlay parent
  `DIR` through observation and publication, takes `UPPER` only below it when
  needed, and relies on underlying helpers for real-parent locks. Successful
  mutation updates or invalidates affected binding/opaque state; a global
  version is not required for correctness.
- **Next stage:** Continue with
  `.agents/designdoc/stageBCdraft/BC-3-merged-directory-readdir.md`.

### B/C-3 Completion Record (2026-07-27, final sign-off 2026-07-28)

- **Status:** Completed and formally signed off during the Stage B/C final
  reconciliation.
- **Artifact:** `.agents/designdoc/stageBCdraft/BC-3-merged-directory-readdir.md`
- **Scope:** `P0-13`, `P0-14`, `P1-31`, and the conditionally retained
  `P2-03`, under `merged_readdir_cache`.
- **Core direction:** Each Overlay directory owns one current, mutable
  ReaddirIndex. The index provides the Overlay cookie namespace for both
  upper-only and merged directories; upper-only source reads may still be
  delegated to the underlying directory, but raw underlying cookies are not
  exposed. FDs retain only the VFS offset and no Overlay cache, snapshot, or
  generation.
- **Cookie direction:** Cookies are monotonic, stable across mutation, and
  never reused. Deletion may use numeric lookup to skip removed entries or a
  successor/tombstone representation. New entries cannot renumber already
  exposed cookies. The readdir return value remains the cursor delta.
- **Projection direction:** ReaddirIndex contains visible entries only.
  B/C-2 BindingCache supplies positive and hidden/negative binding evidence;
  physical whiteouts remain owned by the upper filesystem. `opaque` is a
  directory-level barrier. `impure` is an origin/identity projection hint and
  not a merge visibility authority.
- **Mutation direction:** Successful namespace mutation updates the affected
  BindingCache, barrier state, identity projection, and ReaddirIndex inside the
  same Overlay parent `DIR` transaction. The baseline uses `Valid` and
  `NeedsRebuild` states rather than historical or per-FD versions. Readdir may
  rebuild a dirty index while holding the same `DIR`; partial results are never
  published.
- **Lock direction:** A logical readdir or mutation transaction cannot release
  and reacquire its Overlay parent `DIR`. Sleep-capable underlying BIO is
  allowed within that domain; other locks remain short-lived and ordered.
- **Reconciliation:** The `NeedsRebuild` fallback and same-`DIR` publication
  rule are consistent with B/C-6 mutation semantics and BC-8's final
  physical/semantic publication law. Partial index results are never exposed.
- **Constraint check:** No production Rust, tests, runtime validation,
  Creator/Checker packet, `SYSTEM_BLUEPRINT.md`, or `PASS_SLICING.md` change
  was made.

### B/C-4 Completion Record (2026-07-27)

- **Status:** Completed after interactive refinement; no implementation is
  authorized by this design record.
- **Artifact:** `.agents/designdoc/stageBCdraft/BC-4-copy-up-file-io-page-cache.md`
- **Scope:** The P1 copy-up and file-I/O foundation (`P1-01` through `P1-15`,
  plus `P1-34` and `P1-37`), with `P2-08`, `P2-09`, and `P2-10` retained as
  conditional extensions pending the later Stage D scope decision.
- **Core direction:** One entry-scoped copy-up coordination owner selects the
  winner; workdir temporary state remains private until complete full-data
  staging, metadata/xattr/origin transfer, durability handoff, and physical
  publication. Overlay semantic publication then switches the existing
  logical object to the upper real-file relationship. File views and mmap
  paths delegate to the current authoritative real file, and Overlay does not
  create a second page-cache backend.
- **Context reorganization (2026-07-27):** The draft now starts from one VFS
  request path: classify directory versus non-directory, classify read versus
  write intent, enter the copy-up trigger, obtain the actual real-file/
  real-directory handle or projected directory view, and delegate the
  operation. It explicitly separates regular-file full copy-up, directory
  promotion without child copying, lower-target whiteout, and baseline
  lower-directory rename `EXDEV`.
- **Workdir/page-cache clarification:** Workdir is private staging for regular
  file copy-up rather than a second namespace or cache. Directory promotion
  does not create a workdir data temporary by default. Page cache remains owned
  by the selected underlying inode; the trigger must complete before writable
  mapping or write-capable cache access, and Overlay does not create a second
  page-cache backend.
- **Authority-carrier decision:** The trigger consumes persistent references to
  the existing `OverlayInode`, `OverlayObjectState`, and binding authority. It
  does not return a temporary `AuthorityDecision` carrier. The exact placement
  of in-flight coordination remains intentionally deferred, but completion is
  represented by the published upper binding/authority rather than a permanent
  `copy-up completed` marker.
- **Permission decision:** Overlay-local permission and read-only checks happen
  before any trigger side effect; underlying creator-credential real checks
  happen inside the copy-up operations.
- **Coordination decision:** The copy-up winner retains `CUL` ownership from
  coordination acquisition through semantic publication or failure cleanup.
  Ordinary non-reentrant BIO does not require repeated ready checks. Lookup
  concurrency is serialized by the relevant upper/lower parent locks through
  physical and semantic publication.
- **B/C-2/B/C-3 handoff:** Authority-only file copy-up retains the logical inode,
  name, and directory cookie; xino projection updates raw-identity provenance
  while preserving the required stable `st_ino`/`d_ino` contract. Namespace-
  visible changes alone update or rebuild the ReaddirIndex under parent `DIR`.
- **Failure direction:** Failures before physical publication retain lower
  authority and require explicit temporary cleanup; failures after physical
  publication enter a conservative reconcile path rather than claiming a
  generic rollback or silently publishing an incomplete upper object.
- **Link/object-kind clarification (2026-07-27):** The BC-4 draft now owns the
  copy-up-side rules for lower hardlinks, upper inode reuse, optional
  `PersistentOriginIndex`, hidden-index-link exclusion from overlay `nlink`,
  and ordinary symlink recreation. Read-only hardlink/symlink access does not
  trigger copy-up; namespace mutation publication remains a B/C-6 handoff.
- **Concurrency direction:** The draft retains `DIR -> CUL -> INODE -> WL ->
  UPPER`, forbids file-I/O acquisition of `DIR`, keeps page-cache callback
  inlets lock-neutral, and requires authority revalidation after waits,
  blocking BIO, or potentially reentrant callbacks.
- **Constraint check:** No production Rust, tests, runtime validation,
  Creator/Checker packet, `SYSTEM_BLUEPRINT.md`, or `PASS_SLICING.md` change
  was made.

### B/C-5 Initial Draft (2026-07-27)

- **Status:** A rough, discussion-only draft is now present at
  `.agents/designdoc/stageBCdraft/BC-5-metadata-permission-xattr.md`; the
  stage is not signed off.
- **Scope shape:** The draft treats `P1-16`, `P1-17`, `P1-18`, and `P1-33` as
  the basic metadata/permission/xattr line, and records `P2-05`,
  `P2-06`, `P2-13`, and `P2-14` as conditional extensions without silently
  selecting them for Stage D. Symlink, hardlink, origin/index, and nlink
  object-kind/copy-up mechanics are now documented by B/C-4 rather than this
  permission draft.
- **Core proposal:** Keep real metadata on the selected underlying authority;
  use `OverlayInode`/`OverlayObjectState` for projected authority and
  provenance; apply current-credential local checks before side effects and
  mount-stashed creator-credential real checks unless `default_permissions`
  applies; reuse B/C-4 for lower-backed metadata/xattr copy-up; centralize
  private xattr classification, namespace selection, filtering, and one-level
  nesting escape in `OverlayMetadataPolicy`.
- **Open discussion points:** lower symlink read promotion versus a pinned lower
  link view; whether ACL, fileattr, nlink, `userxattr`, and nested escaping enter
  the basic scope; and the exact Asterinas VFS xattr/ACL/fileattr callback and
  error contracts.
- **Constraint check:** This draft changes only design-tracking documentation
  and the stage index. No production Rust, tests, runtime validation, Creator/
  Checker packet, `SYSTEM_BLUEPRINT.md`, or `PASS_SLICING.md` change was made.

### B/C-5 Pipeline Refinement (2026-07-27)

- **User direction:** Refactor the draft around one public operation pipeline:
  Overlay-local permission check first; only after it passes, obtain the real
  handle/current real authority through B/C-4; then perform the underlying
  creator-credential permission check and delegate the operation.
- **Boundary correction:** B/C-5 does not acquire, own, or redefine the real
  handle/copy-up seam. A local permission failure must not enter B/C-4 or cause
  copy-up. A later real-permission failure retains B/C-4 ownership of any
  already-started transition cleanup/reconcile and does not alter B/C-4's
  copy-up semantics.
- **Xattr correction:** xattr operations add a private/public classification and
  private-owner authorization stage after the local Overlay check and before
  ordinary underlying xattr delegation. A private record that needs persistent
  underlying access still follows the real-handle and underlying-check steps.
- **Scope cleanup:** The BC-5 draft no longer defines symlink-read promotion,
  hardlink copy-up, origin/index behavior, or nlink bookkeeping. It retains
  only the common permission and private-xattr policy handoff for those
  object kinds.
- **Draft update:** The BC-5 artifact was rewritten and shortened around these
  boundaries. Symlink read and hardlink/index mechanics now point to the BC-4
  object-kind section.

### B/C-5 Completion Record (2026-07-27)

- **Status:** Completed and signed off as the metadata/permission/xattr design
  checkpoint.
- **Accepted boundary:** Every operation first performs the Overlay-local
  permission check; only after success does it enter B/C-4 for the current real
  authority, then performs the underlying creator-credential check. A local
  failure cannot acquire a real handle or cause copy-up.
- **Accepted xattr boundary:** xattr operations add private/public
  classification and private-owner authorization between the local check and
  ordinary underlying delegation. Persistent private records still use their
  owning module and B/C-4's real-handle seam when underlying access is needed.
- **Explicit exclusion:** Symlink, hardlink, origin/index, and nlink
  object-kind/copy-up mechanics are owned by the BC-4 discussion boundary;
  B/C-5 only supplies the common permission and private-xattr policy handoff.
- **Constraint check:** No production Rust, tests, runtime validation,
  Creator/Checker packet, `SYSTEM_BLUEPRINT.md`, or `PASS_SLICING.md` change
  was made.

### B/C-6 Completion Record (2026-07-27)

- **Status:** Completed and signed off as the directory-mutation/whiteout
  design checkpoint. The final narrative is in
  `.agents/designdoc/stageBCdraft/BC-6-directory-mutation-whiteout.md`.
- **Scope shape:** The design covers `P1-21` through `P1-30` and `P1-36` under
  `directory_mutations_whiteouts`. `P2-02` `redirect_dir` remains conditional
  on the later Stage D scope decision; the default lower/merged directory
  cross-directory rename remains `EXDEV`.
- **Narrative decision:** B/C-6 is described as one Overlay namespace
  mutation flow rather than a collection of unrelated operation recipes. It
  explicitly distinguishes direct upper operations from workdir staging,
  whiteout preparation, directory promotion, and upper replacement.
- **Workdir decision:** Workdir is private staging on the upper filesystem,
  not a second namespace or readdir source. It participates in copy-up,
  create-over-whiteout, lower-backed whiteout preparation, and selected
  directory replacement paths; ordinary upper-only create and pure-upper
  removal may bypass it.
- **Opaque decision:** Opaque is created only when the operation starts with
  an existing same-name lower directory that is invisible in the Overlay view
  and then materializes a replacement upper directory. A visible lower-only or
  merged directory, or an originally absent name, is not a reason to create
  opaque.
- **Cross-module contract:** The design consumes B/C-1 mount/lifetime policy,
  B/C-2 projection/binding/revalidation, B/C-3 ReaddirIndex/barrier state,
  B/C-4 copy-up and directory promotion, and B/C-5 permission/private-xattr
  policy without creating competing owners.
- **Publication and failure:** Successful physical upper operations publish
  binding, authority, barrier, identity, and directory-index state before the
  affected `DIR` domains are released. Physical/semantic partial failure uses
  conservative invalidation and reconcile; no general multi-object rollback is
  promised.
- **Constraint check:** No production Rust, tests, runtime validation,
  Creator/Checker packet, `SYSTEM_BLUEPRINT.md`, or `PASS_SLICING.md` change
  was made.

### B/C-7 Completion Record (2026-07-28)

- **Status:** Completed and confirmed by the user. The final discussion draft is
  `.agents/designdoc/stageBCdraft/BC-7-advanced-identity-export-data.md`.
- **Direction:** B/C-7 remains a collection of optional widgets, not a new
  namespace or identity owner. It reuses B/C-1 through B/C-6 ownership,
  publication, xattr, and authority-transition boundaries.
- **Confirmed exclusions:** `traps` (`P3-08`) and fs-verity (`P3-05`) are not
  current implementation targets. The basic default for lower/merged directory
  cross-directory rename remains `EXDEV`.
- **Confirmed optional chain:** `origin/index` is optional stage 1;
  `workdir/index cleanup` is optional stage 2; NFS export is optional stage 3
  and depends on the first two. NFS export is one feature boundary: if NFS
  write is not supported, the initial scope does not add a separate NFS read
  implementation.
- **Constraint check:** Only design-tracking documentation and this handoff
  changed. No production Rust, tests, runtime validation, Creator/Checker
  packet, `SYSTEM_BLUEPRINT.md`, or `PASS_SLICING.md` change was made.

### B/C-8 Completion Record (2026-07-28)

- **Status:** Completed by the main agent after reconciling the B/C-1 through
  B/C-7 discussion drafts; the final draft is
  `.agents/designdoc/stageBCdraft/BC-8-cross-module-reconciliation.md`.
- **Unified ownership:** B/C-1 owns mount/layer/upper-workdir runtime state;
  B/C-2 owns projection, binding, identity and revalidation; B/C-3 owns the
  directory `ReaddirIndex`; B/C-4 owns copy-up coordination and authority
  transition; B/C-5 owns local permission and metadata policy; B/C-6 owns
  namespace publication. B/C-7 remains an optional seam and creates no second
  identity, data or page-cache owner.
- **Unified execution law:** All modules use
  `DIR -> CUL -> INODE -> WL -> UPPER`, with normal logical operations holding
  one Overlay parent `DIR` across sleep-capable underlying work. Possible
  re-entry releases and reacquires only after pinned references and full
  revalidation.
- **Unified publication law:**
  `preflight -> physical operation -> physical publication -> semantic
  publication -> ReaddirIndex update/NeedsRebuild`. Partial upper state is never
  a lookup/readdir source, and no generic multi-object rollback is promised.
- **Resolved advanced-feature conflicts:** `xino`, `origin/index`, and NFS
  export are related but distinct; the optional order remains
  `index -> workdir/index cleanup -> NFS export write`. `traps` and fs-verity are
  excluded, `redirect_dir` remains conditional with default `EXDEV`, and NFS is
  not split into a read-only phase.
- **Open follow-ups:** Accepted `identity_and_carriers` and
  `path_lookup_visibility` Designer artifacts need bounded wording repair for
  the one-`DIR` default before implementation scheduling. Stage D now owns the
  selected P2/P3 scope decision and the subsequent comprehensive design review.
- **Constraint check:** Only design-tracking documentation and this handoff
  changed. No production Rust, tests, runtime validation, Creator/Checker
  packet, `SYSTEM_BLUEPRINT.md`, or `PASS_SLICING.md` change was made.

### Stage B/C Final Closure Record (2026-07-28)

- **Result:** Stage B/C is closed. B/C-1 through B/C-8 are recorded as
  complete, with B/C-3 formally signed off and BC-8 accepted as the final
  cross-module consistency checkpoint.
- **Accepted design boundary:** The stage establishes ownership, lifetime,
  authority, visibility, lock order, blocking/re-entry rules, publication and
  conservative reconcile behavior. It does not freeze Rust APIs or authorize
  implementation, pass slicing, or runtime validation.
- **Deferred to later stages:** VFS private-state/publication interfaces,
  selected P2/P3 scope, Designer artifact wording repair, and the comprehensive
  design review. The optional `index → workdir/index cleanup → NFS export write`
  chain remains unscheduled.
- **Next stage:** Stage D scope decision, followed by the comprehensive design
  synthesis/review described in this handoff.

### Stage D Initial Scope Draft (2026-07-28)

- **Artifact:** `.agents/designdoc/stageDdraft.md`
- **Status:** Core scope decisions confirmed; hardlink degradation wording added.
- **Confirmed baseline:** P0/P1 remain mandatory in full: 55 Micro IDs.
- **Initial implementation placement:** Core xino (`P2-01`) is confirmed for the
  P0/P1 wave because it directly affects identity, `stat`, `readdir`, and
  copy-up continuity. Complete UUID modes (`P2-11`) remain a separate decision.
- **High-priority post-core extensions:** `redirect_dir` (`P2-02`), metacopy
  (`P3-03`, with `P3-04` only if data-only layers are needed), and origin/index
  (`P1-07` foundation, `P2-04` verification, `P3-01` index) are proposed after
  basic copy-up, identity, mutation, and page-cache paths stabilize.
- **Late chain:** `P3-09` workdir/index cleanup precedes complete `P3-02` NFS
  export. NFS is not split into a read-only first stage.
- **Default exclusions:** All other P2/P3 features remain deferred; `P3-05`
  fs-verity and `P3-08` traps remain explicitly excluded.
- **Basic hardlink contract:** `P1-28` remains mandatory. An upper-authoritative
  `link()` must preserve sharing of the upper inode, but without `P3-01` index the
  relation among multiple lower hardlink aliases is not guaranteed to survive
  copy-up. `P2-07` nlink preservation, if later selected, does not restore that
  physical aliasing.
- **Constraint:** This draft changes no production Rust, task board,
  `PASS_SLICING.md`, Creator/Checker packet, test surface, or runtime state.

### Stage D Completion Record (2026-07-28)

- **Result:** Stage D scope and implementation timing decision is complete.
- **Accepted baseline:** P0/P1's 55 Micro IDs are mandatory; `P2-01 xino` is
  included in the P0/P1 implementation wave.
- **Accepted post-core priority:** `P2-02 redirect_dir`, `P3-03 metacopy`,
  conditional `P3-04 data-only lower`, and `P3-01 index` remain post-core
  high-priority extensions. `P3-09` precedes any complete `P3-02` NFS export.
- **Accepted hardlink boundary:** Basic `P1-28` preserves upper-authoritative
  hardlinks, but no-index lower multi-link relationships are not guaranteed to
  survive copy-up; `P2-07` would affect reporting/bookkeeping, not restore
  physical aliasing.
- **Default exclusions:** Other P2/P3 features remain deferred; fs-verity,
  traps, and an NFS read-only split remain excluded.
- **Gate result:** Stage-E may proceed as a concise cross-module workflow
  rehearsal; no implementation or pass slicing is authorized by this record.

### Stage E Completion Record (2026-07-28)

- **Artifact:** `.agents/designdoc/stageEdraft.md`
- **Result:** Core workflow rehearsal is complete for mount/root publication,
  lookup/stat/readdir, xino identity projection, file I/O/copy-up, metadata/
  permission/xattr, namespace mutation, hardlink, rename/`EXDEV`, fsync, and
  teardown.
- **Unified closure:** Each workflow is tied to the accepted publication law,
  `DIR -> CUL -> INODE -> WL -> UPPER` lock order, authority transition, and
  conservative reconcile behavior. Pure file I/O is explicitly not required to
  acquire `DIR`.
- **Extension boundary:** redirect_dir, metacopy, index, and NFS export are
  recorded only as future insertion points and are not expanded into Stage-E
  basic workflows.
- **Remaining implementation detail:** Positive/negative VFS private-state
  publication, exact xino fallback/UUID mode behavior, underlying capability
  probes, and extension contracts remain as implementation or extension details;
  they do not block this workflow rehearsal.
- **Constraint:** No production Rust, test surface, Creator/Checker packet,
  `SYSTEM_BLUEPRINT.md`, or `PASS_SLICING.md` change was made.

### Stage F Completion Record (2026-07-28)

- **Artifact:** `.agents/designdoc/stageFdraft.md`
- **Status:** Completed and accepted as the deterministic Stage-F design
  document. BindingCache/ReaddirIndex source-of-truth, VFS handoffs, ordinary
  underlying lookup, and revalidation behavior are closed.
- **Purpose:** Provide the Stage-E workflow to Meso-owner/handoff table and the
  VFS capability closure needed by the Overlay design.
- **Current VFS boundary:** `FileSystem::root_inode()` eagerly returns an
  inode; `Inode::lookup()` returns only an inode; `Dentry` owns name/parent and
  the children cache but no generic filesystem-private payload; negative cache
  entries carry no private state; `lookup_child()` currently calls the
  filesystem lookup while its generic children-cache guard remains active.
- **Confirmed source of truth:** BindingCache is the first source for per-name
  lookup, hidden reason, authority, and binding; ReaddirIndex is the first
  source for visible directory entries and cookies; VFS positive/negative
  dentry cache is a derived path cache and need not carry full Overlay private
  state for basic correctness.
- **Confirmed negative baseline:** `revalidate_absent` may conservatively return
  `false`; the relookup consults BindingCache first and only observes layers on
  a miss or stale entry.
- **Lookup boundary:** When BindingCache misses and underlying BIO is needed,
  ordinary direct underlying lookup may run while the current VFS children-cache
  guard and Overlay parent `DIR` remain held. No release/retry is required for
  this path. Only a proven synchronous callback re-entry or reverse lock order
  requires a pinned-reference adapter and full revalidation.
- **Confirmed closure:** `revalidate_absent` conservatively returns `false`,
  causing a fresh lookup whose first source is BindingCache; ordinary lower-FS
  lookup is not treated as a re-entry hazard.
- **Deferred identity detail:** xino overflow handling is not expanded here;
  concrete behavior may follow the Linux implementation. Full UUID modes remain
  a separate range decision.
- **Constraint:** No production Rust, VFS API, Creator/Checker packet, test
  surface, `SYSTEM_BLUEPRINT.md`, or `PASS_SLICING.md` change was made.

## 9. Closure Record

- Stage A through Stage F design discussion is closed for this handoff.
- No additional traceability or design-review stage is created; current
  traceability is the fresh Architect Macro/Meso mapping plus the staged
  priors and design drafts. No Designer artifact is accepted in this tenure.
- No production Rust, VFS API, Creator/Checker packet, test surface, or
  runtime validation was authorized by the design discussion. The later
  topology reset is the explicit exception that updated `SYSTEM_BLUEPRINT.md`
  and `PASS_SLICING.md`.
- Any future implementation scheduling requires a new main-agent handoff and
  must follow the repository protocol's wording-repair, pass-slicing, and
  Creator/Checker gates.

## 10. Live File Discipline

- **This file is the live handoff for:** 2026-07-24 interactive design tenure
- **Update rule:** Update this same file in place as topics are confirmed,
  deferred, or escalated.
- **Supersedes / Replaces:**
  `20260722-rebase-upstream_main_agent_handoff.md`

## 11. Current Topology Reset Record (2026-07-30)

- **User objective:** Re-split the design into a fresh Macro/Meso topology
  with complete Micro matching, discard the old component split, and prevent
  the accidental Designer wave from being carried forward.
- **Architect dispatch:**
  `.agents/subagent-tasks/architect-reframed-topology/architect_reframed_topology_dispatch.md`
  used an independent Architect context. The packet prohibited production,
  test/runtime, Designer, Creator, and Checker artifacts.
- **Accepted output:** One Macro and seven Meso architecture maps under
  `.agents/components/architect-reframed-topology/`.
- **Structural acceptance:** The seven Meso primary traceability tables contain
  exactly the 81 formal inventory IDs, with no duplicate, missing, or `P3-10`
  row. Every Meso has sections 1-4; the Macro has sections 1-6.
- **Cleanup:** The old `architect-global-topology` and 13 old Meso component
  directories are gone. Their old Designer dispatch packets were deleted;
  protocol templates and historical handoff notes remain as repository
  history/context only. The old baseline component is intentionally retained.
- **Explicit non-actions:** No Designer spec/validation, Creator/Checker/
  Reviewer packet, production Rust, test, build, runtime, or xfstests command
  was generated or run for this reset.
- **Open architecture risks:** The upper/workdir claim carrier remains an
  open choice between an inode-owned `Extension` runtime lease and a
  persistent xattr reservation, as recorded in B/C-1. This does not authorize
  implementation or freeze a VFS API.
- **Next main-agent action:** Keep the board at Architected / Designer not
  started. Any Designer wave requires a new explicit dispatch against the
  seven accepted Meso boundaries.

## 12. Stage-D Micro Scope Classification (2026-07-30)

- **Decision basis:** The Stage-D design document defines the basic
  implementation commitment as all P0/P1 behavior plus the core identity
  extension `P2-01 xino`.
- **需要实现 (56):** all 18 P0 Micro IDs, all 37 P1 Micro IDs, and `P2-01`.
- **暂不实现 (25):** `P2-02` through `P2-17` and `P3-01` through `P3-09`.
  These IDs remain explicit future insertion points in the seven Meso maps;
  their post-core priority or dependency chains do not make them current
  implementation commitments.
- **Meso totals:** `mount_resource_policy` 8/7,
  `visibility_projection_identity` 11/2, `merged_directory_index` 4/1,
  `copyup_authority_file_views` 17/6, `metadata_security_xattr_policy` 4/4,
  `namespace_mutation_whiteout` 11/1, and
  `persistent_association_export` 1/4 (需要实现/暂不实现).
- **Scheduling boundary:** This is a scope classification, not Creator pass
  slicing. No Designer, Creator, Checker, Reviewer, build, test, or runtime
  task is started by this decision. Phase 3 remains unstarted; a future
  Designer wave must be explicitly dispatched under the seven accepted Meso
  boundaries and preserve the 56/25 labels.

### 12.1 Next Main-Agent Actions

1. Keep `SYSTEM_BLUEPRINT.md` at `Architected` / Designer `Not started`.
2. Treat the seven Meso architecture maps and this 56/25 classification as
   the only current topology/scope inputs.
3. Do not create implementation passes until a future explicit Designer wave
   produces accepted contracts and the main agent records actual pass slices.
