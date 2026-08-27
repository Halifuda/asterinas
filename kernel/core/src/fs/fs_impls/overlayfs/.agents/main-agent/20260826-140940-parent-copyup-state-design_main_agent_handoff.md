<!-- SPDX-License-Identifier: MPL-2.0 -->

# 2026-08-26 Parent Pointer and CopyUpState Design Handoff

> **Status (consolidated 2026-08-27). This is the live main handoff** for all
> subsequent overlayfs implementation rounds. The authoritative target design
> is `designdoc/structure-design-proposal-final.md`; this record supplies the
> details the proposal summarizes (parent pointer, copy-up state,
> path-backed `RealObject`). Where the older
> `20260825-143236-final-code-design-decision_main_agent_handoff.md`
> conflicts with this record or with proposal-final — most notably its plan
> to keep a `RealObjectHandle::{Inode, Path}` enum with fallible
> `real_inode()`, and its persistent `RealObjectStack` enum /
> `RealObjectSnapshot` rename — those points are **superseded**; the older
> file stays readable for background (field-order rationale, mount
> assembly) only. Working rule for this record: where the prose abbreviates
> or drifts from current code behavior, code reality wins for describing the
> present (e.g. the cache keeps `rekey_keep_old_alias`, not the docs' plain
> `rekey`).
>
> Provenance: converged design of the 2026-08-26 parent/copy-up round;
> Designer 7's analysis is absorbed into this document, and the surviving
> packets of that round are
> `.agents/components/designer-20260826g/designer-8-three-questions.md` and
> `.agents/components/designer-20260826h/designer-9-lock-order-nocache.md`.
>
> **Amendments 2026-08-27 (accepted study**
> `.agents/components/study-facts-enum-and-project-inode/report.md`**):**
>
> - *(Q2)* `project_inode` is reshaped **in place keeping its name**, taking
>   a two-variant `ProjectionBinding` parameter (`Root` vs
>   `Child { parent, name }`); sections 2.b, 3.2, 3.3, 5.4 and 5.5 below were
>   edited to match. NO renamed sibling `project_inode_with_parent`: a
>   still-generic entry point could mint coordinate-less lower-backed inodes,
>   which is exactly the bug class this rewrite removes. Census basis: of all
>   construction routes only the mount root lacks `(parent, name)` at the
>   call site; its self-parent `Weak` is built by `Arc::new_cyclic` inside
>   the existing cache-create closure (same pattern `OverlayFs::new` already
>   uses for `self_weak`).
> - *(Q1 — deliberately NOT adopted, kept for future explanation)* adding an
>   `Inode` variant to the persistent facts enum is **dropped**. Rationale,
>   in one place so it need not be re-derived: the claimed win does not
>   exist at measurable scale — every upper-state read today is
>   `spin::Once::get()`, an acquire load, which lowers to a plain `MOV` plus
>   a predicted branch on x86_64 (~1 cycle); upper-only objects execute only
>   1–2 such loads per read-class operation and ~4 per mutating operation,
>   against mutex/backend-I/O-scale operations. Reading
>   `UpperOnly(Arc<dyn Inode>)` fares worse: live `RealObject` already caches
>   the real inode (`real.rs:52`), so that payload drops fields needed for
>   cache keys. The cost side is real: ~15 `.upper` and ~15 `.lowers` access
>   sites, roughly seven new accessors plus per-variant constructor and
>   publish branching, and forced re-review of the sensitive
>   rekey-before-`call_once` and winner/waiter paths — against anti-bloat
>   rules forbidding unrelated bundling, and against the death of the old
>   bundling premise (its `RealObjectHandle` partner was superseded by the
>   path-backed, enum-less `RealObject`). If upper-state reads ever show up
>   in profiles, the two found redundancies are fixed directly instead:
>   double `upper.get()` per `read_at`, and a double authority select in the
>   mutating pipeline.

## Decision note

The design now converges on:

- `parent: RwMutex<Weak<OverlayInode>>` — no `Option`. The mount root points to
  itself via a `Weak` obtained from `Arc::new_cyclic`; every non-root points to
  its logical parent / copy-up publication parent.
- `CopyUpState` is `Done | Outstanding(CopyUpTarget)`. `Outstanding` replaces
  all `Active` naming: it covers not-started, in-progress, and needs-repair
  without implying "currently copying up" or "paused/stopped".
- Real locking constraints are only `CUL -> DIR`, `DIR -> PARENT`, and
  `CUL -> PARENT`. `PARENT` and `InodeCache` are short-lived leaf locks; there
  is no meaningful lock order between them, and they must not be nested with
  each other.
- `RealObject` is path-backed. `RealObjectHandle::Inode` /
  `RealObject::identity_only` are removed; `real_inode()` remains infallible via
  a new `RealPath::inode()` that reads `Dentry::inode()` directly.
- The `CopyUpPhase` enum is replaced by a single `need_repair: bool` on
  `CopyUpTarget`. The only currently meaningful phase distinction is whether a
  previous physical publication needs verification before reuse
  (`ReconcilePending`); `Ready` is just `need_repair == false`. Keeping a bool
  avoids a speculative enum. If future features add more phases, the bool can be
  promoted back to an enum at that time.

---

# Designer 7: Parent Pointer and CopyUpState Rewrite

## 1. Problem statement

### 1.1 The current `CopyUpTransition` is awkward

The current per-object coordination is a single `Mutex<CopyUpTransition>` stored on
`OverlayInode`, with this shape (from `copyup/mod.rs`):

```rust
pub(super) struct CopyUpTransition {
    publication_parent: Option<Arc<OverlayInode>>,
    name: Option<String>,
    phase: CopyUpPhase,
}
```

It is awkward for several concrete reasons:

- **`Arc` parent creates a reference cycle with the readdir index.**
  The parent directory's `ReaddirIndex` holds an `Arc<OverlayInode>` for each visible
  child (`ReaddirIndexEntry::Visible { inode: Arc<OverlayInode>, .. }`). If every child
  holds a strong `publication_parent`, then `parent -> child` (via the index) and
  `child -> parent` (via `CopyUpTransition`) form a cycle. The parent can never drop
  while the child is pinned by an index entry, and the index is pinned by the parent,
  so both remain alive forever. A `Weak` parent is required to avoid this leak.

- **Recording is a post-construction `try_lock` write.**
  `OverlayFs::project_inode` constructs an `OverlayInode` with `CopyUpTransition::new()`
  — no parent/name. Only later does `lookup()` call
  `try_record_copyup_transition(parent, name)` using `try_lock`. This creates a window
  in which a lower-backed object is visible but has no copy-up coordinate. It also
  means the first positive lookup has a side effect that can be skipped if the lock is
  contended. In practice the coordinate should already be set before a transition can
  run, but the design remains fragile: if an inode ever reaches `ensure_upper_authority`
  through a path that bypassed `lookup()`, it fails with a mysterious
  `ENOENT: the overlay object has no recorded copy-up publication coordinate`.

- **The type is not entity-named.**
  `CopyUpTransition` is not only a transient transition; it is the object's durable
  copy-up state (`Idle` / `ReconcilePending`) plus publication coordinate. Calling it a
  "transition" obscures that it is a persistent per-object state machine.

- **There is no full, explicit state machine.**
  The actual authority is split across two places: `OverlayInode.upper: Once<RealObject>`
  is the durable "done" signal, while `CopyUpTransition.phase` only knows `Idle` and
  `ReconcilePending`. There is no explicit `NotRecorded`/`Done` in the copied-up state,
  and no single place that documents all states of a lower-backed object.

- **Readdir `..` has no parent pointer.**
  `resolve_parent_object_id` in `readdir.rs` currently resolves the logical parent by
  asking the *real* visible source for `..`, reading lower-id origin records, applying
  determinism gates, and falling back to `d_ino("..") == d_ino(".")`. This is
  expensive, credential-gated in places, and not always exact. A stored overlay parent
  pointer would make `..` a direct `object_id()` lookup on the parent `OverlayInode`.

### 1.2 Why a unified parent pointer is attractive

A single `parent: RwMutex<Weak<OverlayInode>>` on `OverlayInode` can serve both
purposes:

1. **Copy-up publication parent** — `ensure_upper_authority` needs to promote the
   parent first and then publish at `(parent, name)`.
2. **Readdir `..` identity** — the parent's projected `object_id` is exactly the
   identity that `readdir` should report for `..`.

The mount root is the only inode with no logical parent. Instead of encoding that as
`Option::None`, it points to itself via a `Weak` obtained from `Arc::new_cyclic`; this
makes `.. == .` natural and removes `None` branches throughout the code. Every non-root
inode points to the logical parent / copy-up publication parent.

The two notions are conceptually the same for a directory: the namespace parent is the
place where a later copy-up would publish the upper object. Keeping one field means one
place to update on rename, one weak reference to manage, and no invariant that "two
different parent pointers must stay equal."

A `Weak` parent also matches the inode cache's weak-pin model: the inode cache does not
keep parents alive just because a child was once looked up; the readdir index pins
children only while the parent is alive, and nothing in the child should pin the parent
back.

---

## 2. Candidate designs

### 2.a Keep a separate `CopyUpTransition`, but improve it

**Shape:** keep `copyup_transition` separate from any readdir parent pointer, but
change it to:

```rust
pub(super) struct CopyUpState {
    publication_parent: Option<Weak<OverlayInode>>,
    name: String,          // or Option<String> until first lookup
    need_repair: bool,
}
```

and initialize it in the constructor for binding-aware lookups.

**Maintainability.** This is a smaller change than merging into a general `parent`
field, but it leaves two concepts alive: *logical parent for `..`* and *publication
parent for copy-up*. If they are both stored, every rename/redirect must keep them in
sync or the design must document why they differ. The readdir path still needs either
its own parent field or yet another fallback, so the maintainability gain is modest.

**Call-site changes.** The same constructor/`lookup` changes as in 2.b would still be
needed (`lookup.rs`, `copyup/mod.rs`, `inode/mod.rs`). Additionally, `readdir.rs`
would either continue to use the real-parent lookup, or read
`copyup_transition.publication_parent`; the latter couples readdir to copy-up state
and breaks for upper-only objects that have no meaningful transition.

**Concurrency/locking.** Changing `Arc` to `Weak` removes the cycle. Constructor init
can remove `try_record_copyup_transition`. However `ensure_upper_authority` must now
upgrade the weak parent; if it is dead, promotion cannot proceed. That is the same
semantic we need anyway, so this part is fine.

**Future extension.** This candidate is the least attractive for `redirect_dir`: the
copy-up coordinate and the readdir parent pointer are separate, so a future
rename-before-copy-up has to update two fields (or decide which one is authoritative).
It also does not solve the hard-link "which parent?" question any better than 2.b.

**Verdict:** acceptable as an incremental cleanup, but it does not unify the two
naturally identical concepts.

---

### 2.b Merge logical parent and publication parent into one `parent` field

**Shape:**

```rust
pub(super) struct OverlayInode {
    // ...
    parent: RwMutex<Weak<OverlayInode>>,          // root points to itself
    copyup: Mutex<CopyUpState>,                   // name + repair flag; parent lives above
}
```

`CopyUpState` contains only the publication *name* and the repair flag, because the parent is
already on the inode. This is the candidate the rest of this report recommends (with
the concrete state choice from 2.c).

**Root self-parent.** The mount root is created with `Arc::new_cyclic`, and the `Weak`
stored in `parent` is taken from that cyclic allocation. Thus `parent` never needs an
`Option`: the root has `parent.upgrade() == self-strong-reference` while it is alive,
and non-root inodes point to their real parent.

**Maintainability.** There is now one canonical parent field. Rename updates one field;
readdir reads it; copy-up reads it. The invariant is simple: **for an object that is
still lower-backed, `(parent, CopyUpState.name)` is the intended upper publication
coordinate; for a directory, `parent` is also the `..` parent.** The only wrinkle is
that a non-directory lower inode may be reachable through multiple parents (hard links
in lower layers); in that case the field is the *first-seen canonical publication
parent*, which is adequate for the current "first positive lookup wins" copy-up model.

**Call-site changes.**

- `lookup.rs`: reshape `project_inode` in place to take a two-variant
  `ProjectionBinding` parameter, and pass
  `ProjectionBinding::Child { parent, name }` for every positive lookup inside
  `lookup_in_layers`. Remove the `try_record_copyup_transition` call at the end of
  `OverlayFs::lookup`. No renamed sibling function is added.
- `inode/mod.rs`: add `parent`, replace `copyup_transition` with `copyup` state, and
  adjust the lock accessors. Root construction builds through `Arc::new_cyclic`
  inside the cache-create closure when the binding is `Root`, initializing
  `parent` with a self-`Weak`.
- `copyup/mod.rs`: read `parent` from the inode and `name`/`need_repair` from `CopyUpState`;
  the winner/waiter protocol stays otherwise unchanged.
- `readdir.rs`: replace `resolve_parent_object_id`'s real-parent search with a read of
  `self.parent`; retain a weak-parent fallback for the unlikely parent-death case. The
  mount root needs no special `None` case: its self-parent makes `.. == .`.
- `dir/create.rs`: initialise `parent` and `CopyUpState::Done` for the freshly created
  upper objects (call the binding-aware projection with `self` and `name`).
- `dir/rename.rs`: after a successful cross-parent rename, update `parent` (and, in the
  future redirect path, `CopyUpState.name`).
- `dir/link.rs`: no change to `parent` — the first canonical binding remains; the new
  name shares the same upper/overlay inode.
- `inode_cache.rs`: no structural change; it already stores `Weak<OverlayInode>`.

**Concurrency/locking.**

- Constructor init removes the `try_lock` path. A miss under
  `InodeCache::get_or_create` runs the create closure with the binding already known;
  the state is fully initialised before the inode is published in the cache.
- `parent` needs to be mutable only for rename/redirect. A `RwMutex` (sleep-capable,
  not the spin `RwLock`) is appropriate: many readers (readdir, copy-up start) and rare
  writers (rename). `readdir` holds the child's directory transaction lock and then
  reads `parent`; `rename` holds parent directory transaction locks and then writes
  `parent`.
- The relevant lock edges are `CUL -> DIR`, `DIR -> PARENT`, and `CUL -> PARENT`;
  see 3.3 for the exact acquisition patterns.
- `ensure_upper_authority` should read `parent` only long enough to upgrade to an
  `Arc`, then release the `RwMutex` before the recursive ancestor walk, exactly as it
  today clones the `Arc` parent and drops the `copyup_transition` guard before walking.
- For `redirect_dir` later, updating an uncopied object's `(parent, name)` needs a
  deliberate protocol that does not acquire `CUL` under `DIR`. A workable outline is:
  after the logical rename commits, take `parent`-write and `copyup` lock in a narrow
  out-of-band update; or, if the update must be atomic with the rename, take `CUL`
  before the parent `DIR`s. This report only needs to note the hook point; it does not
  need to finalise the redirect protocol.

**Future extension convenience.** This candidate directly supports `redirect_dir`: the
same `parent` field used for `..` is the publication target, so a rename-before-copy-up
only needs to rewrite `parent + CopyUpState.name`. It also gives readdir a cheap exact
answer, and it removes the parent/child strong cycle.

**Verdict:** the cleanest overall structure.

---

### 2.c State machine shape: five-state enum vs `Outstanding + Done` vs `NotRecorded`

#### (i) Full five-state enum

```rust
enum CopyUpState {
    NotRecorded,
    Ready { name: String },
    Promoting { name: String },
    ReconcilePending { name: String },
    Done,
}
```

This makes every phase explicit. However it has real drawbacks:

- `Promoting` is not observable by other threads. The winner holds the `copyup` mutex
  during the entire promotion; by the time any waiter acquires the mutex, the state is
  already `Ready`, `ReconcilePending`, or `Done`. Storing `Promoting` is therefore
  either redundant (written and immediately overwritten while holding the lock) or
  deliberately observable only in panic/debug scenarios.
- It duplicates the `name` in almost every variant unless the variants carry a shared
  `CopyUpTarget` struct. If they do, it is no better than the shape in (ii) and adds an
  extra `NotRecorded` state.
- More states means more transitions to validate and more `unreachable!`/error arms.

#### (ii) Simplified `Outstanding { name, need_repair } + Done`

```rust
pub(super) enum CopyUpState {
    Done,
    Outstanding(CopyUpTarget),
}

pub(super) struct CopyUpTarget {
    name: String,
    need_repair: bool,
}
```

This is the preferred shape. `Done` is the terminal "no further copy-up is needed"
state. It covers both upper-backed objects and the mount root (which is never an upper
copy-up target on a writable mount and cannot be copied up from a read-only mount).
`Outstanding` carries the name and repair flag once; the parent lives in `OverlayInode::parent`.
There is no duplicated target data across variants.

`Outstanding` covers all non-`Done` states without implying "currently copying up" or
"paused/stopped":

- not started → the copy-up is still outstanding;
- in progress → the copy-up is still outstanding;
- needs repair → the copy-up obligation is still outstanding.

#### (iii) Including `NotRecorded`

`NotRecorded` is only useful if there is a realistic path that creates a lower-backed
inode without a `(parent, name)` binding. In the current call graph, the only such
path is the mount root (`OverlayFs::new_root`), which does not need copy-up and can be
represented by `Done`. All other lower-backed objects are produced by `lookup` under a
known parent, and all upper objects are produced by `create`/`copy-up`/`link`, where
the parent is known or no transition is needed.

Therefore `NotRecorded` **can be removed** if we introduce a binding-aware projection
for lookups and make the generic `project_inode` (used by root and upper-object
creation) construct `Done`. We should keep a defensive `debug_assert` in
`ensure_upper_authority` that an `Outstanding` state always has a live parent, and an
`ENOENT`/`EROFS` error if someone reaches an impossible `Done`-but-no-upper state.

**Verdict:** use `Outstanding(CopyUpTarget) + Done`, no `NotRecorded`, no `Promoting`.

---

### 2.d Alternative: dedicated `CopyUpBinding`/parent-pointer object

One alternative is to attach the copy-up coordinate to a *binding* object (a
`(parent, name)` entry) rather than to the inode. This would be closer to Linux's
dentry-oriented model and would handle multiple parents precisely. It would require a
binding cache or per-name entries for each overlay inode, which is a much larger
refactor of the inode-cache/readdir architecture. It also does not reduce the need for
a per-object `parent` for `..` on directories.

For the current state of the code, a binding-level rewrite is over-engineering: the
existing invariant "one lower-backed object copies up through its first-seen parent and
name" is sufficient, and the hard-link split is already explicitly accepted in
`dir/link.rs`. We mention this alternative only to explain why we are not choosing it.

---

### 2.e RealObject simplification: remove `RealObjectHandle::Inode` / `identity_only`

The proposal-side `RealObjectHandle` enum is simplified away. Every real object should
be path-backed:

```rust
pub(super) struct RealObject {
    layer_index: usize,
    path: RealPath,
    fsid: u64,
    container_dev_id: DeviceId,
}
```

There is no `RealObjectHandle::Inode(Arc<dyn Inode>)` variant and no
`RealObject::identity_only` constructor. `real_inode()` remains infallible by deriving
the inode directly from the pinned dentry:

```rust
impl RealObject {
    pub(super) fn real_inode(&self) -> &Arc<dyn Inode> {
        self.path.inode()
    }

    pub(super) fn real_path(&self) -> Result<Path> {
        self.path.upgrade()
    }
}

impl RealPath {
    pub(super) fn inode(&self) -> &Arc<dyn Inode> {
        self.dentry.inode()
    }
}
```

This requires widening `Dentry::inode()` from `pub(super)` to `pub(in crate::fs)` in
`vfs/path/dentry.rs`. It is sound because `RealPath` stores `Arc<Dentry>`, the dentry
strongly owns its immutable `Arc<dyn Inode>`, and the dentry keeps that inode alive
even if the `Weak<Mount>` has already failed to upgrade.

`real_path()` remains fallible (`Result<Path>`) because constructing a `Path` requires
upgrading `Weak<Mount>`; the mount can be gone even when the dentry/inode is alive.

Keeping a cached `real_inode: Arc<dyn Inode>` inside `RealObject` is an acceptable
intermediate step, but the preferred endpoint is no cached inode with infallible
`real_inode()`.

The two historical pathless uses are replaced:

- `OverlayFs::root_visible_key()` derives the root key from `Layer`/`RealPath::inode()`
  directly, not from a pathless `RealObject`.
- `readdir` resolves `..` via `OverlayInode::parent` instead of constructing an
  identity-only `RealObject` to project the parent.

---

## 3. Analysis of the recommended hybrid in detail

This section applies the requested analysis to the recommended design (2.b + 2.c.ii + 2.e).

### 3.1 Maintainability

- **One parent invariant.** `OverlayInode::parent` is the single source of truth for
  both `..` and copy-up publication. There is no "logical parent vs publication
  parent" pair to keep equal.
- **No `Option` in the parent type.** The mount root points to itself, so every inode
  has a parent value. `.. == .` is the natural root behavior and no `None` branch
  exists in the common readdir/copy-up paths.
- **Weak lifetime reasoning is local.** The parent is `Weak`; every use upgrades it
  immediately. The parent is never pinned by the child, so the readdir-index cycle is
  eliminated. The only new reasoning is "what if the parent is gone?" which has a
  defined fallback (`..` self-parent; copy-up error).
- **State is entity-shaped.** `CopyUpState` is named as state, not as a transition.
  `Done` vs `Outstanding` maps directly to the upper `Once` facts and to the single
  name needed for publication.
- **Rename updates are explicit.** After any namespace move, a directory's parent
  pointer is updated. `CopyUpState.name` only needs updating in the future redirect
  case while the object is still `Outstanding`.
- **Real object references are uniform.** Every `RealObject` is path-backed, so
  copy-up/rename/remove paths can always attempt `real_path()`, while identity and
  metadata paths can always use infallible `real_inode()`.

### 3.2 Call-site migration

| File | Current code | New code |
| --- | --- | --- |
| `inode/mod.rs` | `copyup_transition: Mutex<CopyUpTransition>` | `parent: RwMutex<Weak<OverlayInode>>` + `copyup: Mutex<CopyUpState>` |
| `lookup.rs` | `project_inode(...)` then `try_record_copyup_transition(...)` | `project_inode(facts, ProjectionBinding::Child { parent, name })` on all positive lookup branches; delete `try_record...`; no sibling entry point |
| `copyup/mod.rs` | `CopyUpTransition` with `Arc` parent; `try_record`; read coordinate from transition | `CopyUpState`; `ensure_upper_authority` upgrades `self.parent`; `promote`/`finish_promotion` receive the upgraded `Arc<OverlayInode>` and `&str` name; repair semantics carried by `need_repair: bool` instead of the phase enum |
| `readdir.rs` | `resolve_parent_object_id` does real `lookup("..")` and lower-id projection | `resolve_parent_object_id` reads `self.parent` and returns the parent's `object_id()`; root self-parent makes `.. == .` |
| `dir/create.rs` | `project_inode(new_facts)` for new upper objects | `project_inode(new_facts, ProjectionBinding::Child { parent: self, name })` so `parent` and `CopyUpState::Done` are initialised |
| `dir/rename.rs` | promotes source first, then upper renames; no overlay parent update | after successful rename, update `parent` (cross-parent case); future redirect also updates `CopyUpState.name` |
| `dir/mod.rs` | rename admission uses `ensure_upper_authority` before locks | unchanged; but document the new `parent` update in the rename recipe |
| `dir/link.rs` | source promotion; no parent update | unchanged (first canonical parent remains) |
| `inode_cache.rs` | unchanged | unchanged except callers may supply binding info to `get_or_create` create closure |
| `real.rs` | `RealObjectHandle::Inode` / `RealObject::identity_only` / cached inode option | path-backed `RealObject`; `RealPath::inode()`; no `identity_only` |
| `fs/mod.rs` | `root_visible_key()` constructs `identity_only` | derive root key from `Layer`/`RealPath::inode()` directly; no pathless `RealObject` |
| `vfs/path/dentry.rs` | `Dentry::inode()` is `pub(super)` | widen to `pub(in crate::fs)` so `RealPath::inode()` can read it |

### 3.3 Concurrency/locking

#### Real lock-order edges

The overlay-internal ordering constraints are only:

```text
CUL -> DIR
DIR -> PARENT
CUL -> PARENT
```

There is no meaningful total order including `InodeCache`. `PARENT` and `InodeCache`
are short-lived leaf locks; they must be taken and released quickly, and they must not
be nested with each other. In particular, the create closure inside
`InodeCache::get_or_create` may initialize `parent`/`copyup` fields, but it must not
acquire PARENT, CUL, or DIR while holding the cache lock.

#### Initialization

The create closure in `InodeCache::get_or_create` receives the resolved
`ProjectionBinding` on a miss. For a lower-backed child it constructs

```rust
parent: RwMutex::new(Arc::downgrade(parent)),
copyup: Mutex::new(CopyUpState::Outstanding(CopyUpTarget { name, need_repair: false })),
```

and the `ProjectionBinding::Root` case builds the inode through `Arc::new_cyclic`,
whose cyclic weak becomes the self-parent. For an upper-backed
child it constructs `CopyUpState::Done`. The inode is fully initialised before it is
inserted into the cache, so no `try_lock` write is ever needed. For a cache hit, the
existing inode already has its canonical state; no write occurs.

#### Readdir `..`

- **Order:** `DIR(child) -> PARENT(child).read()`.
- `readdir_at_impl` already holds the child directory's `lock`; reading
  `parent` under `RwMutex` follows `DIR -> PARENT`.
- This cannot deadlock with copy-up because copy-up does not hold a child `DIR` while
  acquiring a child `PARENT` read; it reads `PARENT` before taking any `DIR`.

#### Copy-up start

- **Order:** `CUL(child) -> PARENT(child).read()` → release PARENT → release CUL, then
  later `CUL(child) -> DIR(publication_parent)` in `finish_promotion`.
- Concretely: `ensure_upper_authority` first checks `upper.get().is_some()` (fast
  path). If lower-backed, it locks `copyup` (CUL), reads `parent` (PARENT) long enough
  to upgrade the `Weak` to `Arc`, clones `name`/`need_repair`, then drops PARENT and
  CUL before recursively promoting the parent. It later re-acquires CUL as the
  arbitration lock, rechecks upper, and runs `promote` — preserving
  `CUL -> DIR(publication_parent)` in `finish_promotion`.
- **Forbidden inversion:** never hold `PARENT(child)` while waiting on CUL or DIR. If
  copy-up start read `parent` before locking `copyup`, it would create
  `PARENT(child) -> CUL(child)`, which can deadlock against `CUL -> DIR` and
  `DIR -> PARENT`.

#### Rename (current upper-backed source)

- **Order:** `DIR(source parent) -> DIR(target parent) -> PARENT(child).write()`.
- The two parent DIRs are already acquired in `Arc::as_ptr` order. The parent write
  happens after the physical upper rename and while the parent DIRs are held.
- The moved child is `Done`/upper-backed in the current path, so no CUL is needed. Do
  **not** take the moved child’s own DIR just to update `parent`.

#### Future redirect of an `Outstanding` object

- **Order:** `CUL(child)` before `DIR(source parent) -> DIR(target parent) ->
  PARENT(child).write()` → update `CopyUpTarget.name` under the still-held CUL.
- CUL must be acquired **before** the parent DIRs, not after. Taking DIRs and then
  trying to lock `copyup` creates `DIR -> CUL`, the one inversion that deadlocks
  against `CUL -> DIR(publication_parent)`.
- If the update does not need to be atomic with the namespace commit, an out-of-band
  narrow update after releasing DIRs is also acceptable; but when it is inside the
  overlay rename call, the VFS already holds the parent DIRs, so the only safe order is
  CUL-before-DIR.

#### InodeCache

`InodeCache` is a leaf lock with respect to overlay-internal locks. Current copy-up
already does `CUL -> DIR -> InodeCache`, so any path that holds `InodeCache` and then
acquires DIR/CUL/PARENT creates an inversion. The create closure must remain
non-locking.

### 3.4 Future extension convenience

- **`redirect_dir`.** The parent pointer is already the publication target. A future
  rename of a lower-backed directory before copy-up simply updates
  `parent + CopyUpState.name` instead of promoting first. The EXDEV gate in
  `rename.rs` is the place where the policy will choose whether to redirect (update
  the state) or still EXDEV.
- **Readdir `..`.** The parent pointer gives a direct, stable, exact `..` identity and
  removes the fallback-heavy real-parent resolution. The root self-parent removes the
  old `None`-root special case too.
- **Hard links/aliases.** The first-seen `(parent, name)` remains canonical. This is
  consistent with today's "first positive lookup wins" and the accepted split in
  `dir/link.rs`. It does not pretend to track all aliases.
- **Parent death.** A weak parent may be gone; `..` falls back to the existing
  self-parent approximation, and copy-up reports a clear error instead of pinning the
  parent forever.
- **Metacopy/index.** The state shape does not preclude adding more repair-related fields in `CopyUpTarget` later (e.g. a metacopy flag). No speculative abstraction is introduced now.

---

## 4. Edge cases

### Root inode

`new_root` constructs the root with `Arc::new_cyclic`, so its `parent` is a weak
self-reference rather than `None`. If the mount has an upper, the root is `Done` and
`ensure_upper_authority` returns `Ok` through the upper fast path. If the mount is
read-only with no upper, `Done` still represents "not a copy-up target"; any mutating
path is already rejected earlier by the read-only policy, and a hypothetical direct
`ensure_upper_authority` on the root should return a clear `EROFS`/`ENOENT` rather
than attempting to create an upper root under a nonexistent parent. Readdir `..`
naturally returns the root's own `object_id()` because the self-parent upgrades to the
root itself.

### Upper-only objects

Created upper objects use `CopyUpState::Done` and have a `parent` pointer if they are
created under a directory. They never need copy-up; `ensure_upper_authority` hits the
upper fast path. Their `parent` pointer matters only for `..` when they are
directories.

### Lower-backed objects first seen through multiple names/parents

The first binding-aware projection that creates the inode in the cache wins. Later
lookups through other names/parents return the same `OverlayInode` and do not alter
its `parent` or `CopyUpState.name`. For lower hard-link aliases this is the accepted
"first target wins" model; each alias may copy up independently only if the inode is
recreated due to a cache miss, which is the existing limitation documented in
`dir/link.rs`.

### Parent inode destroyed while child still alive

Because the child stores only a `Weak` parent, the parent can drop when no VFS
reference and no readdir index keeps it alive. Consequences are defined:

- Readdir `..`: `resolve_parent_object_id` upgrades `parent`, gets `None`, and uses the
  existing self-parent fallback. For the mount root itself, the self-parent normally
  remains upgradable while the root is alive.
- Copy-up: `ensure_upper_authority` upgrades `parent`, gets `None`, and returns a
  clear error (`EROFS`/`ENOENT`). It never leaks and never resurrects the parent.

### Rename before copy-up (future redirect) vs current rename-after-copy-up

- **Current behavior:** `rename_impl` calls `source_overlay.ensure_upper_authority()`
  before taking the parent locks, so the source is `Done` before the upper rename.
  Rename only needs to update the overlay `parent` pointer for directories (and, for
  completeness, could update it for all objects).
- **Future redirect behavior:** the source may remain `Outstanding`. After deciding to
  redirect, the rename recipe must update `parent` and `CopyUpState.name` so a later
  copy-up publishes into the new location. This requires acquiring CUL before the
  parent DIRs or using a narrow post-lock update; see 3.3. The current design leaves
  that hook in `dir/rename.rs`.

### Concurrent first lookups of the same child

Two tasks may look up the same lower-backed child under different parents concurrently.
`InodeCache::get_or_create` serialises creation; exactly one create closure runs and
initialises `parent`/`CopyUpState`. The other task gets the cache hit and does not
overwrite the canonical binding. There is no `try_lock` and no uninitialised window.
If the child is being copied up by a winner, the `copyup` mutex arbitrates as today;
the winner already has an `Outstanding` state because it was created with a binding.

---

## 5. Recommendation

### 5.1 Chosen design

Use a **hybrid of 2.b, 2.c.ii, and 2.e**:

1. Add one unified `parent: RwMutex<Weak<OverlayInode>>` field to `OverlayInode`; the
   mount root uses `Arc::new_cyclic` and points to itself.
2. Replace `CopyUpTransition` with `CopyUpState = Done | Outstanding(CopyUpTarget)`.
3. Eliminate `NotRecorded` by initialising every lower-backed lookup result through
   the reshaped in-place `project_inode(facts, ProjectionBinding)` entry point;
   root and upper-only creations are `Done`.
4. Keep the winner/waiter `copyup` mutex as today, but read `parent` from the field
   and `name` from `CopyUpState`.
5. Remove `RealObjectHandle::Inode` / `RealObject::identity_only`; make `RealObject`
   path-backed with infallible `real_inode()` through `RealPath::inode()`.

### 5.2 Concrete type sketch

```rust
pub(super) struct OverlayInode {
    // ...existing fields...
    /// Canonical overlay parent. The mount root points to itself via a Weak
    /// obtained from Arc::new_cyclic; non-root inodes point to their logical
    /// parent / copy-up publication parent. Weak avoids the parent<->child
    /// cycle with ReaddirIndex.
    parent: RwMutex<Weak<OverlayInode>>,
    /// Copy-up coordination for lower-backed objects.
    copyup: Mutex<CopyUpState>,
}

pub(super) enum CopyUpState {
    /// Terminal: no copy-up is needed (upper-backed, mount root, or read-only root).
    Done,
    /// Copy-up has not yet completed. The object is still lower-backed; it may
    /// not have started, may be in progress, or may require verification/repair
    /// after a partial physical publication.
    Outstanding(CopyUpTarget),
}

pub(super) struct CopyUpTarget {
    /// Name used when publishing into the parent's upper directory.
    name: String,
    need_repair: bool,
}

pub(super) struct RealObject {
    layer_index: usize,
    path: RealPath,
    fsid: u64,
    container_dev_id: DeviceId,
}

impl RealObject {
    pub(super) fn real_inode(&self) -> &Arc<dyn Inode> {
        self.path.inode()
    }

    pub(super) fn real_path(&self) -> Result<Path> {
        self.path.upgrade()
    }
}
```

### 5.3 Key invariants

1. `parent` is always a `Weak`; no child ever strongly pins a parent.
2. The mount root's `parent` upgrades to the root itself. Every non-root inode created
   from a positive lookup or create recipe points to its logical/publishing parent.
3. If `CopyUpState::Outstanding`, then the inode is lower-backed (`upper.is_none()`),
   and `(parent, CopyUpTarget.name)` is the fully recorded publication coordinate.
4. If the inode is upper-backed, `CopyUpState` is `Done` (or transitions to `Done`
   atomically inside the winner's critical section).
5. First binding wins: once `CopyUpState` is `Outstanding` or `Done`, later lookups do
   not modify it.
6. `ensure_upper_authority` never takes `copyup` while holding a parent directory
   transaction lock; `finish_promotion` remains the single `CUL -> parent-DIR` edge.
7. `parent` for a directory is updated on successful cross-parent rename; in future
   redirect mode, the same update also refreshes `CopyUpTarget.name` for an
   `Outstanding` object.
8. `RealObject` is always path-backed; `real_inode()` is infallible, `real_path()` is
   fallible.

### 5.4 Initialization/update rules

- **Miss in `project_inode(facts, ProjectionBinding::Child { parent, name })`:**
  create the `OverlayInode` with `parent = Arc::downgrade(parent)` and
  `copyup = if facts.upper.is_some() { Done } else { Outstanding(CopyUpTarget { name, need_repair: false }) }`.
- **Root (`ProjectionBinding::Root`):** build through `Arc::new_cyclic` so the root's
  `parent` is the resulting self-`Weak`; `copyup = Done`.
- **Hit:** return the existing inode; do not write state.
- **Upper-backed child creations (create / copy-up carrier):** same `Child`
  binding as lower-backed ones; because the facts are upper-only,
  `copyup = Done`.
- **Rename (current):** after the physical upper rename succeeds, update the source
  directory's `parent` to the new parent. Since the source is already upper-backed,
  `CopyUpState` is `Done`; no CUL is needed.
- **Rename (future redirect):** before treating a rename as a redirect, update
  `parent` and `Outstanding(CopyUpTarget).name` under the lock protocol described in
  §3.3.

### 5.5 Main call-site migrations

1. `inode/mod.rs` — new fields and accessors; delete `try_lock_copyup_transition`
   (or keep only if needed for future diagnostics).
2. `lookup.rs` / `fs/mod.rs` — reshape `project_inode` in place into
   `project_inode(facts, ProjectionBinding)` with variants `Root` and
   `Child { parent, name }`; route all positive lookup branches through it;
   delete the post-lookup `try_record_copyup_transition`. No sibling function
   is added: a still-generic variant could mint coordinate-less lower-backed
   inodes (census 2026-08-27: the mount root is the only construction site
   that lacks a binding).
3. `copyup/mod.rs` — rename/reshape the state; update `ensure_upper_authority_inner`,
   `promote`, `finish_promotion`, and `mark_reconcile_pending`.
4. `readdir.rs` — replace `resolve_parent_object_id` with the `parent`-based
   implementation; root self-parent removes the `None`-root special case.
5. `dir/create.rs` — initialise parent/state for new upper objects.
6. `dir/rename.rs` — update `parent` on successful cross-parent rename; reserve the
   future redirect hook.
7. `dir/mod.rs` / `dir/link.rs` — mostly unchanged; only update comments and any
   direct calls to `project_inode`.
8. `real.rs` — remove `RealObjectHandle::Inode` and `RealObject::identity_only`; make
   `RealObject` path-backed; add `RealPath::inode()`.
9. `fs/mod.rs` — derive `root_visible_key()` from `Layer`/`RealPath::inode()` instead
   of `identity_only`.
10. `vfs/path/dentry.rs` — widen `Dentry::inode()` to `pub(in crate::fs)`.

### 5.6 Room for future features without over-engineering

- `redirect_dir` only adds a rename-time target update for `Outstanding` objects; the
  data model already has the needed `parent` and `name`.
- Readdir `..` is simplified to an in-memory projection, eliminating fragile
  real-backend resolution.
- Hard links/aliases remain first-seen-wins, matching current behaviour and the
  accepted limitation in `dir/link.rs`.
- Metacopy/index features can extend `CopyUpTarget` without reshaping
  `OverlayInode`'s ownership model.
- No binding cache, per-name state, or dentry-level rewrite is introduced now; that
  remains a possible later evolution if multiple-parent tracking ever becomes
  necessary.

---

## Implementation scope (2026-08-27, complete change list)

Verdicts of the accepted study (`.agents/components/study-facts-enum-and-project-inode/report.md`)
are binding: no `Inode` variant in the persistent facts enum (Q1), and
`project_inode` is reshaped in place with a `ProjectionBinding` parameter,
no sibling function (Q2). Where a step below cites a doc that conflicts with
code, code reality wins (e.g. `rekey_keep_old_alias` stays; the inode cache's
`get_or_create` contract is unchanged and the create closure at
`inode/lookup.rs:273` is the only one).

### Batch A — Layer strong-mount + path-backed RealObject

1. `layer.rs` — `Layer` becomes `{ mount: Arc<Mount>, root_dentry: Arc<Dentry>,
   fsid, container_dev_id }` (replaces `root_path: RealPath` + `fs: Arc<dyn FileSystem>`);
   delete all `.upgrade()?`/`.expect(...)` at `layer.rs:54,57,100,144`.
2. `real.rs` — `RealObject` becomes `{ layer_index, path: RealPath, fsid,
   container_dev_id }`; drop cached `real_inode` and `Option<RealPath>`; delete
   `identity_only()` (`real.rs:65`); `real_path()` loses its `None` arm; add
   infallible `RealPath::inode()` reading `Dentry::inode()`.
3. `vfs/path/dentry.rs` — widen `Dentry::inode()` from `pub(super)` to
   `pub(in crate::fs)` (`dentry.rs:297`).
4. `fs/mod.rs` — rewrite `root_visible_key()` (`fs/mod.rs:70`) to derive the key
   from `Layer`/`RealPath::inode()` directly; the two `identity_only`
   constructions at `fs/mod.rs:72,89` disappear.
5. `fs/mount/layer_parts.rs` — `Layer::resolve_parts` produces
   `(Arc<Mount>, Arc<Dentry>)` inputs for `Layer` (`layer_parts.rs:126,130,158,161,169,172`).
6. `fs/mount/mod.rs` — adapt the four `upper.root_path.upgrade()?` uses at
   `mount/mod.rs:77,84,88,97` to the new `Layer` shape.
7. `inode/mod.rs` — `new_root`'s two layer-root upgrades at `inode/mod.rs:110-127`
   become direct reads.
8. `inode/identity.rs` — `collect_layer_devs` adapts its `Layer` reads at
   `identity.rs:94,109` (was `.root_path.upgrade()?.inode().ino()`).

### Batch B — parent pointer + CopyUpState + ProjectionBinding (one coupled pass)

New shapes (types in `inode/copyup/mod.rs` / `inode/lookup.rs`):

```rust
// OverlayInode gains:
parent: RwMutex<Weak<OverlayInode>>,   // mount root points to itself, no Option
copyup: Mutex<CopyUpState>,            // replaces copyup_transition: Mutex<CopyUpTransition>
enum CopyUpState { Done, Outstanding(CopyUpTarget) }
struct CopyUpTarget { name: String, need_repair: bool }  // replaces CopyUpPhase
enum ProjectionBinding<'a> { Root, Child { parent: &'a Arc<OverlayInode>, name: &'a str } }
```

1. `inode/copyup/mod.rs` — delete `CopyUpTransition`, `CopyUpPhase` (all 11 uses:
   `:57,66,83,90,127,224,429,638` and docs); `mark_reconcile_pending` (`:637-643`)
   becomes a `need_repair = true` write; `try_record_copyup_transition` (`:114-136`)
   is deleted together with its internal `try_lock` use (`:119`);
   `lock_copyup_transition` uses at `:159` (read coordinate in
   `ensure_upper_authority_inner`) and `:185` (in `promote`) are replaced by
   locking `copyup` and reading `self.parent` (upgrade to `Arc`; `None` -> clear
   error); `promote`/`finish_promotion` take the upgraded `Arc<OverlayInode>` +
   `&str` name; the copy-up carrier projection at `:598` passes its known
   publication coordinate as `ProjectionBinding::Child`.
2. `inode/lookup.rs` — reshape `project_inode(&self, facts, binding)` in place
   (`:214`); the four positive-hit branches (`:113,118,150,179`) pass
   `Child { parent, name }`; the create closure (`:273`) holds the single
   binding match — `Root` builds via `Arc::new_cyclic` (self-parent `Weak`),
   `Child` initializes `parent`/`copyup` per §5.4; delete the
   `try_record_copyup_transition` call at `:192`.
3. `inode/mod.rs` — add `parent`/`copyup` fields and accessors; delete
   `lock_copyup_transition` (`:202`) and `try_lock_copyup_transition` (`:208`);
   `new_root` (`:98-134`) delegates to `project_inode(facts, ProjectionBinding::Root)`.
4. `inode/inode_cache.rs` — no structural change; only the create-closure
   contract is exercised with the binding.
5. `inode/dir/create.rs` — both projections (`:78`, `:138`) pass
   `Child { parent: self_arc, name }`; upper-only facts yield `Done`.
6. `inode/dir/rename.rs` — after the successful upper rename (holding the two
   parent DIR locks), write the moved object's `parent` to the new parent;
   reserve the future-redirect hook at the EXDEV gate (comment only).
7. `inode/dir/link.rs` — unchanged.
8. Lock-order invariants written into these files: `CUL -> DIR`, `DIR -> PARENT`,
   `CUL -> PARENT`; no `DIR -> CUL`; InodeCache stays a leaf lock; the create
   closure acquires no locks (pure field init allowed).

### Batch C — readdir `..` via the parent pointer

1. `inode/readdir.rs` — replace `resolve_parent_object_id` (`:314`, called at
   `:103`) with a `DIR(child) -> PARENT(child).read()` read of `self.parent`,
   returning the parent's `object_id()`; root self-parent gives `.. == .`
   naturally; keep the existing self-parent fallback for parent-death; the
   `identity_only` construction at `:366` disappears with the old resolver.
   `readdir.rs:437` (root-key consumption) is unchanged.
2. Optionally route the lone raw `lowers[0]` escape (`readdir.rs:277`) through
   the existing accessor.

### Batch D — finish (independent, low risk)

1. Field ordering to the mixed scheme: `OverlayFs` =
   `layer_stack, policy, identity, upper_workdir_pair, _anon_device_id,
   whiteout_cache, inodes, fs_event_stats, self_weak`; `OverlayInode` =
   `fs, lowers, upper, object_id, lock, parent, copyup, extension`.
2. Q1-found redundancies fixed directly: double `upper.get()` per `read_at`
   (`inode/data.rs:32` + `inode/mod.rs:143`); double authority select in the
   mutating pipeline (`inode/permission.rs` ~`:130,187`). The triple check in
   copy-up arbitration (`copyup/mod.rs:149,190,214`) is intentional — keep.

### Explicitly out of scope

Persistent facts enum / `Inode` variant (Q1 verdict); inode-cache internal
semantics and `rekey` (code keeps `rekey_keep_old_alias`); whiteout / workdir /
capabilities / options / identity projection internals (already match
proposal-final); metacopy, redirect_dir, index (hooks only); no code changes
are part of the 2026-08-27 documentation commit.

---

## 2026-08-27 implementation record: Batches A–D executed and accepted

User authorized scheduling this day. All four batches ran as serial
command-free Creator passes (Direct Spawn Lane) with main-agent exact-diff
structural acceptance; **no compile/lint/runtime command was run** (container
closed). Slicing recorded in `PASS_SLICING.md`
(`parent_copyup_batches_slicing_20260827`) with two code-grounded re-slices:

1. **RealObject reshaping moved from Batch A to pass_48**: `identity_only`'s
   last live caller is the old readdir resolver, which itself needs Batch B's
   parent field; deleting it in A would have broken the intermediate tree.
   §2.e's "acceptable intermediate step" covers the same carve-up. A ships
   only `RealPath::inode()`.
2. **Parent-Arc resolution frozen** (avoiding write-set growth): `OverlayFs::lookup`
   resolves the parent's canonical `Arc<OverlayInode>` through the inode cache
   before projecting (the pre-existing "live parent registered under its key"
   invariant; miss now fails closed with EIO instead of log-and-skip), so the
   six non-Arc `fs.lookup(self, …)` call sites needed zero edits;
   `dir/create.rs` uses a new private `OverlayInode::cached_self_arc()` at its
   two projection sites.

Accepted outcomes per batch (receipts + snapshots under
`components/parent-copyup-state-design/`):

- **pass_46_layer_strong_mount**: `Layer{mount, root_dentry, fsid,
  container_dev_id}` strong-mount redesign; all layer-root upgrade/expect
  sites eliminated across layer.rs / layer_parts.rs / mount/mod.rs /
  fs/mod.rs / new_root / collect_layer_devs; `Dentry::inode()` widened to
  `pub(in crate::fs)`.
- **pass_47_parent_copyup_state**: `CopyUpState/CopyUpTarget`,
  `ProjectionBinding<'a>`, `OverlayInode.parent/copyup`, root self-parent via
  `Arc::new_cyclic`; winner commits `Done` under the guard; commit-failure arm
  sets `need_repair = true`; carrier projection binds Child; rename writes
  parent under held DIR locks; EXDEV gate keeps the redirect hook comment.
  Deviations D1–D8 verified; D1 (read-only grep/find usage against the letter
  of the dispatch ban) recorded as non-blocking process deviation.
- **pass_48_readdir_parent_identity**: `..` resolved by reading
  `self.parent` (`.. == .` at root; dead weak → self fallback); old resolver
  chain, `root_visible_key`, `identity_only`, cached `real_inode`, and
  `Option<RealPath>` deleted with zero live remnants; RealObject endpoint
  `{layer_index, path, fsid, container_dev_id}` reached. Main agent applied
  one mechanical fix (trailing comma in fs/mod.rs import), recorded in the
  receipt's acceptance note.
- **pass_49_field_order_and_q1_redundancy**: mixed-scheme field ordering for
  both structs; read_at double-Once merged into one snapshot select; the
  permission.rs double authority select adjudicated as NOT collapsible
  (two-stage admission spans promotion — intentional); zero-edit deliverable.

Next-main-agent actions:
1. When the container opens: packeted Checker compile gate first
   (target-specific `cargo check -p asterinas --target x86_64-unknown-none`),
   then workspace lint; fix path follows PROTOCOL rule 10.
2. Runtime revalidation after green compile: the schedulable regression table
   (`002 003 006 007 010 011 012 014 024 031 038 077`) — copy-up coordinate
   internals and `..` identity sources changed, so run the full table once,
   not only previously-failing cases.
3. Commit decision for the four batches' working tree belongs to the user
   (default proposed: one commit per accepted batch using the archived
   snapshots as boundaries, or a single squashed implementation commit).
4. Reviewer gate over the cumulative diff can run before or after the
   compile gate (static lane, command-free); recommend after compile to keep
   review deltas stable.

## 2026-08-27 static gate closed: `make check` fully green

User opened the container and directed the Checker lane. Three rounds of
`task_checker_parent_copyup_compile_lint_20260827`
(single gate: `docker exec -w /root/asterinas codex-asterinas-dev make check`):

- **run01 FAIL** at the front gates before compilation: 4 trailing-whitespace
  markdown lines (PASS_SLICING.md, structure-design-appendices.md ×2,
  structure-design-proposal.md — all main-agent-owned `.agents` records) plus
  9 rustfmt hunks in the four-batch code. Main agent applied all mechanically.
- **run02 FAIL** with 4 clippy errors (first time compilation/clippy was
  reached): 3× `inconsistent_struct_constructor` from pass_49's field
  reordering (fixed by reordering constructor literals per clippy's own
  suggestion), and dead-code on `IdentityPolicy::is_all_layers_same_fs` /
  `is_directory_projection_deterministic`. The Checker tagged the latter as
  design; the main agent adjudicated it mechanical with evidence — grep
  proved zero live callers and their only consumers were already deleted in
  the ACCEPTED pass_48 design (old readdir resolver chain), so deletion merely
  completes pass_48's "no dead code" frozen rule one file beyond its
  write-set. Both methods deleted; the stale doc clause on `xino_fits`
  referencing the deleted determinism route was tightened too.
- **run03 PASS** — exit 0 in ~21 s, full pipeline (rustfmt, clippy ×2 gates,
  kernel member, non-default members, bzimage setup, regression-test format
  gate) with zero diagnostics.

Evidence:
`components/parent-copyup-state-design/run_evidence/checker_compile_lint_20260827/run{01,02,03}/`;
receipt: `components/parent-copyup-state-design/task_checker_parent_copyup_compile_lint_20260827_checker.md`.
Checker never self-repaired across all rounds; every diagnostic preserved
verbatim. Non-blocking anomaly recorded: run03 reported PROTOCOL.md missing
at three candidate paths — the file exists at the overlayfs workspace root
(read-verified this session); treated as subagent path-resolution noise.

Remaining gates (pending user instruction): runtime xfstests revalidation
(the schedulable 12-case table, full run since copy-up coordinates and `..`
identity sources changed) and the commit decision for the uncommitted working
tree.

## 2026-08-27 proposal-review design notes (recorded ONLY; no code changes)

Outcome of the peer-review explanation rounds and two user meetings, recorded
as scheduling-menu input for later sessions. **Nothing below is implemented or
dispatched; user explicitly froze code changes ("不要改代码").**

### 1. Proposal revision plan (ready-to-write, awaiting green light)

Full draft appendix was authored by the main agent and appended verbatim by a
background subagent to
`designdoc/structure-design-proposal-final-review.md`
(注意：该文件此轮已被 user 改名——更早会话读到的是
`structure-design-proposal-final-peer-review.md`；若改名非用户所为需回迁核实)。
Ten planned items reduce to three blocks:

- 新增：动机节 overlayfs 一句话定义；§7 规则区 whiteout/opaque 定义下放 +
  三条一行 case；NegativeLookup 三态行为契约（对外统一 ENOENT、内部分支差异）。
- 定约（规范源，实现将向其收敛）：`parent` 字段改名 +
  rationale 三层事实（构造期一次性绑定与跨父 rename 唯一重写点；
  多 alias first-seen-wins 与 accepted split 从实现注释升格为文档承诺；
  不由物理 dentry 反查派生——逻辑命名空间演化快于物理形态，redirect 式
  语义是根本理由）。字段名候选 `binding_anchor` / `publication_parent` /
  `publish_parent`，**待用户最终拍板**。
- 精简：§5 root_dentry 注释最简重写（clone-mount 可行性论证全部留在本节，
  不进文档）；§6 RealPath 收敛至两三行；§2 字段块对齐 pass_49 终态并
  删组序口号；L136 UB 句只挂 Linux overlayfs.rst reference（用户指示不引
  自有工程文档）；章节顺序按评审建议调整为 层模型紧跟挂载流程。
  en 版是否同步翻译待定。

### 2. Copy-up 四操作的方法化定约（规范形状）

§9 四个自由函数改为 `impl OverlayInode` 四方法（lock_copyup /
stage_in_workdir / publish_by_rename / copy_up），类型闭合要点：
`MutexGuard` 出借可变目标经 `CopyUpState::outstanding_mut()`（need_repair
合法写入的最小闭环）；need_repair 置位归 publish_by_rename 内部、Done 提交
归 copy_up 成功尾部；祖先逐级提升游离于四抽象之外由外层机制承担；workdir
作为挂载级资源藏于 stage 抽象背后，不把 OverlayFs 引入伪代码。这是规范性
目标形状，后续实现 wave 将把生产代码向它收敛——尚未调度。

### 3. 瞬时事实栈：文档一句话 + 实现侧备忘

- 文档侧已定稿单句（进 §7 流程第 1 步后）："合并扫描把各层命中项就地移动
  收集为瞬时事实栈，热路径不做克隆、不引入额外分配。"——**不出现**容器
  类型/move 关键字/机制解释（会议定调）。
- 实现侧备忘（同属"stack 的 move 消费"，均未调度）：借用视图不可行
  （child hit 是每次解析的新产物无处可借），根治形态为值内联：
  ① `layer.rs` 三个构造器与 `lookup.rs:114` 的 `dir_hits` 换
  `SmallVec<[RealObject; 4]>`（workspace 已有 smallvec 1.13.2，ext2 有
  使用先例）；② 把 `lookup.rs:284-285` 的 eager
  `facts.lowers/upper.clone()` 下移进 create closure，竞态命中路径零拷贝。

### 4. Dentry 化转发 + 复用 clone_mount（本轮讨论的主设计项，全部未授权）

实证基础：Mount 相等 EXDEV 门只存在于 `Path::rename`(vfs/path/mod.rs:708)
与 `Path::link`(:676)；深层 `DirDentry::rename`(dentry.rs:762) 无凭证检查、
无 mount 参与；`as_dir_dentry_or_err` 已 `pub(in crate::fs)` 且 overlay 今日
已在用 `DirDentry::lookup_child`。非 dir 操作审计结论：数据/xattr/metadata/
symlink 等一贯经 `select_real_inode()` 直发 `Arc<dyn Inode>`，不经过 Path，
零受影响面；真实爆破半径恰为目录条目变更族 + workdir 工具带。

既定两刀切法：

1. **第一刀（纯机械）**：仅放宽 rename/mknod/new_fs_child/link/unlink/rmdir
   至 `pub(in crate::fs)` 并换调 DirDentry 层。单一共享 Mount 下所有判定
   逐位不变（门被绕过但本就不触发），可独立编译+回归验收。
2. **第二刀**：**不新增任何 VFS API** —— 复用既有
   `Mount::clone_mount(root_dentry, new_ns)`(mount.rs:454，共享 fs、根任指、
   parent/mountpoint 天然空)，仅需一处可见性放宽到 `pub(in crate::fs)`；
   以空 `Weak<MountNamespace>` 获得不对拓扑注册的孤儿视图（构造器注释的
   pseudo-mount 先例即权威依据）。产三组视图（lower×n、upper、workdir 各一）
   后：`Layer` 收缩为 `{mount, fsid, container_dev_id}`（root 由 clone 承载）、
   `RealObject` 只存 `Arc<Dentry>` + 身份三元组、`RealPath` 类型整体删除，
   锚点有效性不变式转为 "OverlayFs 活 ⇒ 视图活"，EIO-on-upgrade 分支族消亡。
   被跳过的 `check_dir_entry_mutation`（mount-writable 标志 + 凭证复查）须在
   packet 中显式签收为设计决策。

遗留审计点（第二刀 packet 内完成）：空弱 ns 手法对照、20 处消费点的 C 类
映射表（含 capabilities/inuse/workdir 工具带）、flags 对 lower 克隆只读语义
的处理、workdir 第三视图的边界（可能不在 upper 子树内）。

Next-main-agent actions（新增部分覆盖式并入上节的遗留清单）：
1. proposal 十项修订落笔前的两个开放输入：parent 字段名拍板；en 版同步与否。
2. 若授权实施上述任一项（方法化收敛 / stack 内联 / 两刀转发改造），走
   bounded Designer packet 冻结面后切片派发；在此之前维持零代码改动状态。

### 5. 导师反馈两则的处置记录（2026-08-27 晚）

- **RealObject 收敛**：`fsid`/`container_dev_id` 从 `RealObject` 移出、经
  `layer_stack[idx]` 取用的方向记录在案；"Arc<dyn FileSystem> 指针 + ino 做
  cache key"实验**否决**（fsid 即该身份的最简编码）。后经用户确认按本方向
  **落地到正文**：§6 的 `RealObject` 现为 `{layer_index, dentry}`，
  层身份由层定义统一携带；实现侧收敛在后续 wave 跟随。
- **need_repair 说明修正**：正文 §9 该段已按"窗口定义 + 两类真实失败源
  （ENOSPC 写盘错误 / 并发查找抢先占缓存位）+ 复用协议"重写；overlay
  inode_cache 的 displacement 分支语义与之相符。
- **binding_anchor 的 trait/Dentry 替代方案**：导师提出的议题明确搁置
  （波及 Inode trait 七口签名 + 全 fs 适配，工作量大）；此前评估中的关键
  观察（overlay 逻辑树即 VFS namespace dentry 树、副本视图使层根/身份可从
  单一判别信息推导）保留在本节供日后重启。相关补丁三冲突/四开放问题
  维持在 `proposal_dentry_clone_view_patch_20260827.md`。
