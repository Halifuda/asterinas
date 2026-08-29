<!-- SPDX-License-Identifier: MPL-2.0 -->

# 2026-08-26 Parent Pointer and CopyUpState Design Handoff

> **Status (consolidated 2026-08-27). This is the live main handoff** for all
> subsequent overlayfs implementation rounds. The authoritative target design
> is `designdoc/structure-design-proposal-final.md`（round4 终态，含 en 译本；
> 需注意 proposal 的 recorded_parent/copyup/dotdot 表述已领先于代码，差距
> 清单见文末「proposal 终态 vs 代码现状」节）; this record supplies the
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
> `rekey`); 本文件的章节按时间分层——文末各节（implementation record /
> static gate / design notes / 终态 vs 代码现状 / 2026-08-28 决策记录）为最新权威，
> 覆盖前文与终态冲突之处。
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

Next-main-agent actions（2026-08-27 晚更新：1/3 已完成，见下节 static gate
记录与本日晚间提交；2/4 仍有效）：
1. ~~compile/lint gate~~ ✅ 已完成（见下节 static gate closed）。
2. Runtime revalidation after green compile: the schedulable regression table
   (`002 003 006 007 010 011 012 014 024 031 038 077`) — copy-up coordinate
   internals and `..` identity sources changed, so run the full table once,
   not only previously-failing cases. ⚠️ 注意：下文 §6 的实现 backlog 落地后
   需再跑一轮，届时以终态代码为准合并执行。
3. ~~Commit decision~~ ✅ 已完成：`2f868281e`（round1–4 代码主体）＋
   `37bf37676`（proposal/review 记录与本轮 doc 修订，含评审文件入档）。
4. Reviewer gate over the cumulative diff 仍待运行（static lane，
   command-free）——建议与 backlog 实施合并后一次性执行。

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

Remaining gates（2026-08-27 晚更新）: ~~commit decision~~ ✅ 已提交
（`2f868281e` 代码主体、`37bf37676` 记录/文档与评审文件入档）；runtime
xfstests revalidation 仍待用户指令，且应与 §6 backlog 的实现合并后统一
执行一轮。

## 2026-08-27 proposal-review design notes (recorded ONLY; no code changes)

Outcome of the peer-review explanation rounds and two user meetings, recorded
as scheduling-menu input for later sessions. **Nothing below is implemented or
dispatched; user explicitly froze code changes ("不要改代码").**

### 1. Proposal revision plan（✅ 已全部执行完毕，本节仅存档）

本节所述十项计划及后续追加轮次（round2 R1–R20、round3 批注修正与终态并入、
round4 need_repair 退场 + `recorded_parent` 更名与探讨节）**全部落地正文**，
收据与 manifest 见
`components/parent-copyup-state-design/proposal_pending_changes_manifest_20260827.md`。
两处开放输入均已关闭：字段名终定为 **`recorded_parent`**（用户裁定，
`binding_anchor`/`publication_parent`/`publish_parent` 候选作废）；en 版已
删除旧译并按中文版忠实重译（commit `0e18923b2`）。本节以下原文仅作历史存档：

- 新增：动机节 overlayfs 一句话定义；§7 规则区 whiteout/opaque 定义下放 +
  三条一行 case；NegativeLookup 三态行为契约（对外统一 ENOENT、内部分支差异）。
- 定约（规范源，实现将向其收敛）：`parent` 字段改名 +
  rationale 三层事实（构造期一次性绑定与跨父 rename 唯一重写点；
  多 alias first-seen-wins 与 accepted split 从实现注释升格为文档承诺；
  不由物理 dentry 反查派生——逻辑命名空间演化快于物理形态，redirect 式
  语义是根本理由）。字段名候选 `binding_anchor` / `publication_parent` /
  `publish_parent`，~~待用户最终拍板~~。
- 精简：§5 root_dentry 注释最简重写（clone-mount 可行性论证全部留在本节，
  不进文档）；§6 RealPath 收敛至两三行；§2 字段块对齐 pass_49 终态并
  删组序口号；L136 UB 句只挂 Linux overlayfs.rst reference（用户指示不引
  自有工程文档）；章节顺序按评审建议调整为 层模型紧跟挂载流程。
  en 版是否同步翻译待定。

### 2. Copy-up 四操作的方法化定约（⚠️ 已被终态取代，仅存档）

本节定约描述的是中间形态（CopyUpState/CopyUpTarget 仍存在、含 need_repair
闭环）。最终形态已再进一步：`need_repair` 整体删除后，`CopyUpState`/
`CopyUpTarget` 一并退场，坐标并入不可变载体，仲裁锁不再携带状态——
权威表述以正文 §9 与下文「proposal 终态 vs 代码现状」节为准。

~~§9 四个自由函数改为 `impl OverlayInode` 四方法（lock_copyup /
stage_in_workdir / publish_by_rename / copy_up），类型闭合要点：
`MutexGuard` 出借可变目标经 `CopyUpState::outstanding_mut()`（need_repair
合法写入的最小闭环）；need_repair 置位归 publish_by_rename 内部、Done 提交
归 copy_up 成功尾部；祖先逐级提升游离于四抽象之外由外层机制承担；workdir
作为挂载级资源藏于 stage 抽象背后，不把 OverlayFs 引入伪代码。这是规范性
目标形状，后续实现 wave 将把生产代码向它收敛——尚未调度。~~

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

### 6. proposal 终态 vs 代码现状（implementation backlog，下一轮代码 wave 的既定范围）

正文（round4 后）已是规范终态；代码停留在 batches 46–49。差距逐项如下，
全部**未调度**，实施前按惯例出 bounded Designer packet 冻结面：

- **a) copyup 机制收敛**：代码现状为 `CopyUpState{Done,
  Outstanding(CopyUpTarget{name, need_repair})}` ＋ `binding_anchor:
  RwMutex<Weak>`；终态为 `recorded_parent: RwMutex<Weak>`（更名）＋
  `copyup: Mutex<Option<String>>`（仅发布名，发布完成置 None 退役），
  `CopyUpState`/`CopyUpTarget`/`need_repair`/repair 核验链整体删除，
  四方法改共享借用签名（正文 §9 冻结文本为准）。
- **b) RealObject 收敛**：代码现状 `{layer_index, path: RealPath,
  fsid, container_dev_id}` ＋ RealPath 类型；终态 `{layer_index,
  dentry: Arc<Dentry>}`，路径经 `LayerStack[idx]` 重建，RealPath 删除；
  `identity.rs` 对 `real.fsid()/real.container_dev_id()` 的消费点改走层取用。
- **c) dotdot**：代码的一行式 resolver 已读锚点并回落自身，语义与正文一致；
  仅需跟随 a) 完成更名，无独立工作。
- **d) 来源记录前移**：代码 `store_lower_id` 仍位于发布之后
  （`publish_upper_authority` 内）；按正文语义移入制备段（写于 workdir
  临时体），失败即普通清理路径——与 a) 同批实施。
- **e) 登记接纳式汇合**：代码 `rekey_keep_old_alias` 遇同物活体占位仍
  报错；按正文契约改为接纳（等待复用／接替完成），并把正主活性判定同步
  放宽（避免"名字上的对象不是我"造成的重试死结）。与 a) 同批。
- **f) 事实栈内联备忘**（SmallVec 值内联＋eager clone 下移）照旧挂起。

已知继承性局限（正文已背书，实现无需动作）：别名分裂行为面（含读侧
过时与二次全量复制）、未发布目录的 EXDEV 拒绝。

Next-main-agent actions：
1. a)–e) 打包为一轮 bounded Designer packet（冻结 need_repair 删除面、
   RealObject 收敛面、来源记录前移与接纳汇合的精确条件），随后切片派发；
   f) 可并入或延后。
2. §4 的两刀转发改造维持冻结，待接口面决策；dotdot 的 per-open 视图路线
   已并入该议题族。
3. en 版已完成（`0e18923b2`）；文档侧无遗留。
4. 若实现期间需要重新核对设计依据，以正文（round4 版）为准，本 handoff
   的早期章节（Decision note/§1–§5/Implementation scope）仅作历史存档。

### 5. 导师反馈两则的处置记录（2026-08-27 晚）

- **RealObject 收敛**：`fsid`/`container_dev_id` 从 `RealObject` 移出、经
  `layer_stack[idx]` 取用的方向记录在案；"Arc<dyn FileSystem> 指针 + ino 做
  cache key"实验**否决**（fsid 即该身份的最简编码）。后经用户确认按本方向
  **落地到正文**：§6 的 `RealObject` 现为 `{layer_index, dentry}`，
  层身份由层定义统一携带；实现侧收敛在后续 wave 跟随。
- **need_repair 终局（覆盖本节下方早期记录）**：经两轮推演（触发模型修正 +
  别名分裂反例），最终裁定 `need_repair` **从设计中整体删除**——来源记录
  写入前移至制备阶段、登记处对同物占位采取接纳式汇合之后，物理改名后的
  收尾不存在失败类。正文已按此重写完毕（round4，收据
  `task_doc_creator_proposal_final_apply_round4_20260827_report.md`）；
  早期"窗口定义 + 两类失败源"版本已废弃，实现侧历史记录不构成回退依据。
- **binding_anchor 的 trait/Dentry 替代方案**：导师提出的议题明确搁置
  （波及 Inode trait 七口签名 + 全 fs 适配，工作量大）；此前评估中的关键
  观察（overlay 逻辑树即 VFS namespace dentry 树、副本视图使层根/身份可从
  单一判别信息推导）保留在本节供日后重启。相关补丁三冲突/四开放问题
  维持在 `proposal_dentry_clone_view_patch_20260827.md`。

## 2026-08-28 user scheduling decisions（dentry 接口 PR 拆分、struct 约束边界；仅记录，未执行）

背景：本日会话完成「proposal 终态 vs 代码现状」的逐行 gap 复核（handoff §6 backlog
(a)–(f) 全部经代码验证仍成立；新增两处核对发现——`OverlayFs` 字段序 drift、
`get_or_create` 谓词正文未覆盖，见下），并确认 copy-up 顺序缺陷：`store_lower_id`
在 `publish_upper_authority` 内、位于物理 rename **之后**写 origin xattr
（`inode/copyup/mod.rs:560`，失败即 `need_repair`），与 Linux 在 rename 前
于 workdir temp 上写 origin（`copy_up.c:689`）相悖——(d) 前移 + (e) 接纳汇合
即 need_repair 全链消除的完整证据链已入档本会话。同日用户会议裁定如下，
**全部为调度约束，不改任何代码、不派发任何 packet**：

1. **dentry 代入 inode 接口 = 独立 PR；合入前 overlayfs 事实上不做此事。**
   proposal §5「VFS 侧替代方案」（dentry 承载发布坐标、删 `recorded_parent`）与
   §4 的第一刀转发（DirDentry 进 rename/link/unlink 等 fs 方法签名）整体冻结，
   等待该平台 PR 合入；在此之前 `recorded_parent` 保持规范坐标载体，任何切片
   不得假设 dentry 携带型调用。§6 Next-actions 第 2 条的「待接口面决策」即此
   决策本身，维持冻结不变。
2. **mount（clone-view）与接口 PR 是两件事。** 第二刀的私有挂载视图装配
   （proposal §3 构造流程第 2 步、§4 `Layer{mount, fsid, container_dev_id}`
   三字段、§6 `RealObject{layer_index, dentry}` 收敛）不依赖该 PR，与转发改造
   解耦，可独立调度——即 backlog (b) 不受决策 1 阻塞；但其遗留审计点
   （空弱 ns 手法、20 处消费点 C 类映射、flags 只读语义、workdir 第三视图
   边界、`check_dir_entry_mutation` 跳过签收）仍在 packet 内完成。
3. **proposal 的 struct 严格遵循；impl 视为讲解简化，不必贴合。** §2/§4/§5/§6/§7/§8
   的类型块（含 `recorded_parent: RwMutex<Weak<OverlayInode>>`、
   `copyup: Mutex<Option<String>>`、`RealObject`/`Layer`/`OverlayFs` 字段块与
   字段序、`Lookup`/`NegativeLookup`）为硬约束。此裁定同时：① 使 §2 字段序
   drift 成为代码侧待修项（`_anon_device_id` 移至 `fs_event_stats` 之后）；
   ② **修订 §6(a) 原文「四方法改共享借用签名（正文 §9 冻结文本为准）」的力度**——
   §9 的 `copy_up()` 等代码体降为示意，方法分解/签名/锁持有细节由未来
   Designer packet 自行冻结（`copyup: Mutex<Option<String>>` 的 struct 形态
   仍为硬约束）。
4. **`get_or_create` 的 `is_same_object` 谓词必要性存疑**（proposal §8 无此
   参数）：ino 复用陈旧占位驱逐能否在无谓词契约下由其他机制覆盖，列为未来
   Designer packet 的裁决项。配套澄清：`rekey_keep_old_alias` 的旧 key 别名
   保留按 §5 别名分裂规则推定为必要（fresh lower-only lookup 依赖其判
   stale-upper 重建），正文 §8 `rekey`「迁移」表述需在 packet 中澄清为
   「新 key 挂接 + 旧 key 别名保留」。
5. 本节与「proposal 终态 vs 代码现状」节共同构成下一轮 wave 的既定范围基线；
   实施前仍按惯例出 bounded Designer packet（决策 3/4 为其新增输入）。运行时
   xfstests 全表回归与 Reviewer gate 继续押后至 wave 收尾一次执行。

## 2026-08-29 xattr 小节压缩定案与交付物一重写（已验收，纯文档）

用户复核 `xattr-design-and-gaps_20260828.md` 交付物一（原 X.1–X.6 约 240 行）后裁定：
proposal `## 设计` 的容纳单元是**小节**（各节 10–42 行），xattr 定案为新开
`### 13. 扩展属性 inode/xattr.rs`（插于现 §12 之后；现 §13 去掉 xattr 行改题
「权限、属性、数据」顺延为 §14，现 §14 顺延为 §15；proposal 内部无 §13/§14 交叉
引用，改号零成本）。写作笔法硬要求：**以类型/接口叙述、必要时伪代码**（锚点
§5/§7/§9），不得通篇散文。已派 `task_doc_creator_xattr_chapter_condense_20260829`
（packet：
`subagent-tasks/doc-xattr-condense-20260829/task_doc_creator_xattr_chapter_condense_20260829_dispatch.md`）
完成重写并验收通过：proposal-ready 正文 40 行（enum `XattrName`/`XattrPrefix` +
text 伪代码块承载两条写路径与转义管线，散文仅承担动机/推论/宁拒勿错）；原讲解
材料无损移入交付物一内「附：机制详解与例子」annex（附 A–E，Linux file:line 全部
收容于此）；交付物二/三正文未动。主代理补一处越界面遗留（header 交付物三条目
§14→§15）。修订轮 2（同日，用户六点反馈，packet 已追加修订轮 2 节，同一 doc
creator 续做，验收通过）：opaque/whiteout/origin 括注删除（impure 保留极简括注），
no-goals 机制与互斥段移出正文（素材留附 C），trusted 展开移至前缀参数段，
userxattr 前置括注，加段机制配单层最小例与嵌套结论，降级段改为直陈；
`XattrName::Plain` 更名 `Passthrough`（与术语 plain 名撞词）；正文仍 40 行。
「转义管线」一词按用户措辞偏好全文件清除（annex 标题与交付物二/三标签由主代理
机械替换为转义规则/转义缺失/转义机制）。修订轮 3（同日，用户四点反馈，packet 已
追加修订轮 3 节，同一 doc creator 续做，验收通过）：删「与 Linux 同构」句、正文
不再挂外部引用；「私有命名空间」改述为「overlay 私有前缀」；两条路径改用两个
简洁伪代码块承载（读写不分类、例子进注释、list 隐藏/剥段收进路径二注释、嵌套
结论保留为一句散文、中缀细则留附 B）；`XattrPrefix` 前加类型引入句、变体裸列；
降级两族清除 no-goals 字眼（被拒族删 redirect/EXDEV 项，降级族改「以标记为前提
的增强能力整体关闭」）。正文仍 40 行。修订轮 3 补（同日，用户三点小改，主代理
直改未派 subagent）：路径二定义为「经 mount 进来的一切请求」（嵌套时上层 overlay
的请求也在其中）；trusted 句简化为「写它需要 CAP_SYS_ADMIN 特权」；userxattr
改「有两种用处」并补全句子成分。同日新任务
`task_doc_creator_xattr_gaps_language_20260829`（packet:
`subagent-tasks/doc-xattr-gaps-language-20260829/…_dispatch.md`，fresh spawn）
对交付物三做全面语言修订并验收通过：发明术语正常化（正确性半径→出错波及范围、
内部 IO 权威/写权威/作用域覆盖/平面 EXDEV/redirect 配方/CUL/DIR/仿射记账/
兄弟名字/灌数据/元数据权威等逐条处置，CUL/DIR 展开为 copy-up 互斥锁/父目录
事务锁，两段式准入对齐 proposal 的两段式权限检查）；成分省略全面补全（redirect
流程改编号步骤、CUL/DIR 缩写展开、谓语与指代补全）；机制事实、全部 file:line、
四段式结构未变；主代理验收另清「写意图」→「以可写方式打开」（2 处）。遗留观察：
原稿 4 条 Linux 事实（CAP_SYS_RESOURCE 配额动机、index 项转 whiteout、先发布
index 后硬链次序、params.c metacopy 依赖）未做源码级复核，与 dentry PR 相关的
recorded_parent 双表述经 2026-08-28 决策 1 已自洽，均无需动作。Next：下一轮文档
修订把 `### 13.` 正文
并入 proposal final 并执行顺延；交付物二 gap 清单并入 live handoff 的范围基线。

## xattr 现状 vs 设计 gap 清单（2026-08-29 并入）

> 本节并入自 `xattr-design-and-gaps_20260828.md` 交付物二（当前 xattr 代码 vs 正确设计的 gap 清单，2026-08-28）；与「proposal 终态 vs 代码现状」节同为下一轮 wave 的范围基线输入。技术语言；行号以 2026-08-28 当前工作树复核为准。

### 现状基线（结构性事实，非缺口）

- 内部标记写走直连真身的专用路径：`set_impure_marker` 文档明示不经过用户面拒绝面
  （`inode/xattr.rs:354-370`）；`set_opaque_marker` 能力门 fail-closed 返回
  `EOPNOTSUPP`（`inode/xattr.rs:387-407`）；whiteout 的两形态由两个独立探针门控
  （`fs/mount/capabilities.rs:43-45,90-107,120-122`）。
- syscall 层 Trusted 门的读写不对称已复刻：set/remove 无能力返回 `EPERM`
  （`syscall/setxattr.rs:229-251`、`syscall/removexattr.rs:66`），get 把同一拒绝
  映射为 `ENODATA`（`syscall/getxattr.rs:98`）。
- 来源记录存储/读取的编解码与能力门已就位（`inode/identity.rs:487-543`）。

### 规则缺口

- **R1** `syscall/setxattr.rs:229-251`：`check_xattr_namespace` 只对 `Trusted`
  设门，`user.*` 的"仅普通文件与目录、sticky 目录仅属主/特权者可写"两条规则整链
  缺失（Linux `fs/xattr.c:158-176`）；ext2 的 `set_xattr` 是纯存储实现、无命名
  空间规则可依赖；tmpfs 无 xattr 面（trait 默认 `EOPNOTSUPP`，
  `fs/vfs/fs_apis/inode.rs:554-573`）。规则必须在 syscall/VFS 层补齐而不是依赖
  具体后端。

### 转义缺失

- **E1** `inode/xattr.rs:463-531`：get/set 的拒绝面直接拦下 own 前缀名；没有
  "own 前缀 + `overlay.` + 后缀" 的下行拼接转义（Linux `xattrs.c:148-171,173-209`），
  经 mount 的 own 前缀写读不能穿透到 backing。
- **E2** `inode/xattr.rs:159-185`：`filter_private_names` 只隐藏、不剥段——转义名
  应剥前缀后第一段再上行展示，本实现一律按私有名剔除或保留原名。
- **E3**（E1/E2 的嵌套后果）：内部标记写虽走直连真身路径（`xattr.rs:354-370`），
  但嵌套时"真身"是下层 overlay 的逻辑 inode，其用户面 `set_xattr_impl` 会对
  plain 名按 Private 拒绝——同前缀叠加的"按段数分层、各自命中"隔离在当前实现中
  无法建立。

### userxattr 缺失

- **U1** `inode/xattr.rs:61,73,79,86`：写侧四个标记全名常量硬编码
  `trusted.overlay.*`（origin/opaque/whiteout/impure），前缀未参数化。
- **U2** `inode/xattr.rs:58,132`：`USER_OVERLAY_PREFIX` 只活在分类器里，不参与任何
  写路径；`fs/mount/options.rs` 无 `userxattr` 选项。
- **U3** `fs/mount/capabilities.rs:36-51`：能力探针固定用 trusted 前缀名
  （经 `uuid_xattr_name`，`xattr.rs:61`），不随命名空间选择探针；亦无
  userxattr 与 redirect/metacopy 的互斥校验（后者目前本就未实现，校验规则需随
  选项一并预留）。

### 策略分歧（Linux 对照）

- **P1** `inode/xattr.rs:129-144`：classify 的实现态 quirk——Linux 形态转义名
  （`trusted.overlay.overlay.X`）先命中 `trusted.overlay.` 分支，后缀 `overlay.X`
  不在已知表（`:51-54`）中，落 **Reserved** 而非 Escaped；Escaped 臂
  （`:70,139`）只接字面 `overlay.overlay.` 开头的名字，对 Linux 形态名基本不可达
  （此类名也过不了 VFS 的命名空间解析，永远不会到达 classify）。
- **P2** `inode/xattr.rs:463-531` 与 `:159-185`：Linux 对 own 前缀名**无拒绝类**——
  已知/未知/已转义一律转义透传，保护靠命名空间错位而非拒绝；本实现对
  Private/Reserved/Escaped 全部拒绝并隐藏，Reserved 类比 Linux 严，属兼容性差异。
- **P3** `inode/xattr.rs:475-481`：get 拒绝码 `EOPNOTSUPP` 与 Linux 可观察行为
  （ENODATA，读侧隐藏）不符；set/remove 的 `EPERM` 作拒绝码惯用，分歧在"拒绝"
  行为本身而非错误码。`has_marker` 对 `ENODATA`/`EOPNOTSUPP` 都映射"无标记"
  （`xattr.rs:322-340`），改码无内部破坏。

### 杂项

- **M1** `inode/xattr.rs:503-512`：`list_xattr_impl` 的 `MAY_ACCESS` 权限需求是
  占位（DAC 块尚未评估 `MAY_ACCESS`），list 的读类权限未真正生效。
- **M2** `syscall/setxattr.rs:234-238`：Trusted 门取 `permitted_capset()`，而
  能力计值基线（`process/credentials` 的 capable 语义，packet 引
  `capability.rs:34-38`）按 effective 集计——两处 capset 口径不一致；Linux
  `capable()` 查 effective（`kernel/capability.c:414`）。

### 凭据缺口

- **G1** `inode/xattr.rs:191-194`：`copy_eligible_xattrs` 文档自认无 creator
  credential 作用域，源读与 temp 写都运行在调用者凭据下（Strict 策略因此会把
  EACCES/EPERM 升级为整次操作失败）。同一根因波及 copy-up 的元数据/时间戳转移
  （`inode/copyup/mod.rs` 的 promote 制备段）与 clear-empty 的 xattr 复制
  （`inode/dir/remove.rs:249-252` 注释已绕开其中一处时序问题）。Path 层
  （`fs/vfs/path/mod.rs:279-283` 的 `check_dir_entry_mutation`、`:763-795` 的
  xattr 准入）与 DirDentry 层（`fs/vfs/path/dentry.rs:471-489` 的 sticky 检查）
  残留调用者凭据依赖，是 overlay 内部 IO 无法以"挂载者权威"运行的 VFS 侧面。
  机制、场景与改点见交付物三第 1 条（credential 缺口）。

## 2026-08-29 晚：proposal 合并/审阅/en 同步与 xattr §13 用户重写（纯文档，均验收）

1. `task_doc_creator_xattr_merge_20260829`：三交付物逐字并入——交付物一→proposal 新
   §13（旧 §13 删 xattr 行改题「权限、属性、数据」顺延 §14，§14 顺延 §15），交付物三→
   §16 Gaps and no-goals，交付物二→handoff 文末「xattr 现状 vs 设计 gap 清单」节；
   0828 加归档记号。主代理随修 gap 清单内 `##`→`###` 标题层级。
2. `task_reviewer_proposal_readability_20260829`：proposal 全文可读性直改 16 处（协议
   指称自洽化、Strict/BestEffort 未定义标签删除、体例统一）；主代理回退其把 §13 两路径
   伪代码改散文的一处（用户轮 3 明令伪代码承载）。
3. `task_doc_creator_proposal_en_sync_20260829`：en 重建至与 zh 逐句一致（362→457 行）；
   en 节引用全部改 GitHub issue compatible `#N`（17 处与 zh `§N` 一一对应）；zh 无需微调。
4. 用户大幅重写 en §13（两路径改为 `impl OverlayInode` 伪代码：私有路径
   `set_overlay_xattr` 直传、透传路径 `set_xattr_impl` 加段）。主代理对照真实代码修正：
   分类枚举定名 `XattrClass{Private,Passthrough}`（对齐 inode/xattr.rs，避免与 VFS struct
   `XattrName` 撞名）；两方法签名保持 `name: XattrName`（与 VFS `Inode::set_xattr` 接口
   一致，真实类型为 struct{namespace, full_name}）；`mul→mut`、`start_with→starts_with`、
   `value/flag→value_reader/flags`；中缀插入用真实 String 接口
   `insert_str(selected_prefix.len(), "overlay.")`（注释带前后对照例）；
   `into()` 证伪——`String`→`XattrName` 无 `From` 实现（类型借用 `&str`），改真实构造器
   `XattrName::try_from_full_name(&used_name).ok_or(Errno::EINVAL)?`。
5. `task_doc_creator_xattr_en_backport_20260829`（后台）：en §13 回填 zh §13 并使 0828
   交付物一正文与之逐字一致（含 insert_str / try_from_full_name 两条中途增量）；验收：
   diff 为空、无 stale 代码形态、镜像删除清单确认。
观察项：handoff gap 清单与 0828 annex/交付物二仍用「own 前缀」旧措辞，与正文新「私有前缀」
并存，未统一，待用户定夺；当前 proposal/designdoc/handoff 改动均未提交，待用户指示后
amend 进 WIP `1d5bbd53d`。
收尾（同日晚）：经用户裁定删除三份过时 designdoc——`structure-design-proposal.md`、
`structure-design-proposal-revised.md`、`xattr-design-and-gaps_20260828.md`（xattr 三交付物
内容已全部并入 proposal §13/§16 与本 handoff，annex 背景材料随之退役）；amend 时
commit message 去除 WIP 前缀。
