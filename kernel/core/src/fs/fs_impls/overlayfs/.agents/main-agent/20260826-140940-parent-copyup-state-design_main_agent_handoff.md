<!-- SPDX-License-Identifier: MPL-2.0 -->

# 2026-08-26 Parent Pointer and CopyUpState Design Handoff

> This handoff records the converged overlay parent-pointer and copy-up state
> design after the Designer 7, 8, and 9 discussions. The existing
> `20260825-143236-final-code-design-decision_main_agent_handoff.md` remains
> the active consolidated handoff for the proposal revision; this is a separate
> design record for the parent/copy-up rewrite and the related `RealObject`
> simplification.

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

- `lookup.rs`: add a binding-aware projection entry, e.g.
  `project_inode_with_parent(parent, name, &facts)`, and call it for every positive
  lookup inside `lookup_in_layers`. Remove the `try_record_copyup_transition` call at
  the end of `OverlayFs::lookup`.
- `inode/mod.rs`: add `parent`, replace `copyup_transition` with `copyup` state, and
  adjust the lock accessors. Root construction uses `Arc::new_cyclic` to initialize
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
- `readdir ::` uses `OverlayInode::parent` instead of constructing an identity-only
  `RealObject` to project the parent.

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
| `lookup.rs` | `project_inode(...)` then `try_record_copyup_transition(...)` | `project_inode_with_parent(parent, name, ...)` on all positive lookup branches; delete `try_record...` |
| `copyup/mod.rs` | `CopyUpTransition` with `Arc` parent; `try_record`; read coordinate from transition | `CopyUpState`; `ensure_upper_authority` upgrades `self.parent`; `promote`/`finish_promotion` receive the upgraded `Arc<OverlayInode>` and `&str` name; keep `ReconcilePending` logic |
| `readdir.rs` | `resolve_parent_object_id` does real `lookup("..")` and lower-id projection | `resolve_parent_object_id` reads `self.parent` and returns the parent's `object_id()`; root self-parent makes `.. == .` |
| `dir/create.rs` | `project_inode(new_facts)` for new upper objects | `project_inode_with_parent(self, name, new_facts)` so `parent` and `CopyUpState::Done` are initialised |
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

The create closure in `InodeCache::get_or_create` gets access to `parent` and `name`
on a miss. For a lower-backed child it constructs

```rust
parent: RwMutex::new(Arc::downgrade(parent)),
copyup: Mutex::new(CopyUpState::Outstanding(CopyUpTarget { name, need_repair: false })),
```

and the root constructor uses `Arc::new_cyclic` for the self-parent. For an upper-backed
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
3. Eliminate `NotRecorded` by initialising every lower-backed lookup result through a
   binding-aware projection; root and upper-only creations are `Done`.
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

- **Miss in `project_inode_with_parent`:** create the `OverlayInode` with
  `parent = Arc::downgrade(parent)` and
  `copyup = if facts.upper.is_some() { Done } else { Outstanding(CopyUpTarget { name, need_repair: false }) }`.
- **Root construction:** use `Arc::new_cyclic`; the root's `parent` is the resulting
  self-`Weak`, and `copyup = Done`.
- **Hit in `project_inode_with_parent`:** return the existing inode; do not write
  state.
- **Generic `project_inode` (root/create/copy-up carrier):** create with
  `parent` as appropriate and `copyup = Done`.
- **Rename (current):** after the physical upper rename succeeds, update the source
  directory's `parent` to the new parent. Since the source is already upper-backed,
  `CopyUpState` is `Done`; no CUL is needed.
- **Rename (future redirect):** before treating a rename as a redirect, update
  `parent` and `Outstanding(CopyUpTarget).name` under the lock protocol described in
  §3.3.

### 5.5 Main call-site migrations

1. `inode/mod.rs` — new fields and accessors; delete `try_lock_copyup_transition`
   (or keep only if needed for future diagnostics).
2. `lookup.rs` — introduce `project_inode_with_parent`; route all positive lookup
   branches through it; delete the post-lookup `try_record_copyup_transition`.
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
