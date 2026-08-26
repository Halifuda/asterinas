<!-- SPDX-License-Identifier: MPL-2.0 -->

# 2026-08-25 Final Code Design Decision — Overlayfs Proposal Revision

## 1. Purpose

This handoff consolidates the parallel read-only explorations and Designer
studies about the overlayfs code-design discussion into one final decision.
It is the authoritative summary for revising
`kernel/core/src/fs/fs_impls/overlayfs/.agents/designdoc/structure-design-proposal.md`.

The temporary exploration/designer report directories under `designdoc/` have
been removed after this summary was written; this handoff is the surviving
record of those conclusions.

## 2. Overall Position

The proposal will be revised as a **re-implementation design**, not merely a
code-cleanup proposal. It must:

- explain the legacy implementation's concrete deficiencies;
- introduce the overlayfs semantics a reader needs before the design;
- present a coherent target type landscape;
- avoid over-engineering and code/type bloat.

The team has decided to bundle the following design-level changes because the
goal is to revise the proposal at the design level. If later implemented, they
should still be landed in the safe order in §6.

## 3. Final Design Decisions

### 3.1 Proposal framing and semantics

- Rewrite the proposal to lead with a short overlayfs semantic glossary:
  layer stack/order, merged directory, whiteout, opaque, copy-up, upper-first
  merge-stop lookup, two-step permission, direct-underlying-modification UB,
  inode identity reuse, workdir staging + atomic publication.
- Add a legacy-deficiency section grounded in the frozen legacy:
  single-file monolith, concurrency races, no inode identity reuse,
  non-persistent identity, incomplete rename/sync/sb/metacopy/workdir/readdir/
  permission features.
- Map each structural choice to a legacy deficiency so the proposal reads as a
  reimplementation rationale.

### 3.2 Field order

- Use the **mixed ordering** scheme:
  1. core immutable state;
  2. synchronization state;
  3. caches/resources/weak refs.
- `OverlayFs` recommended order:
  `layer_stack`, `policy`, `identity`, `upper_workdir_pair`, `_anon_device_id`,
  `whiteout_cache`, `inodes`, `fs_event_stats`, `self_weak`.
- `OverlayInode` recommended order:
  `fs`, `stack`, `object_id`, `extension`, `lock`, `copyup_transition`.
- Proposal code blocks and actual code should match in field set and order;
  comment prose may differ.

### 3.3 Layer, RealPath, RealObject

Final target shapes:

```rust
// layer.rs
pub(super) struct Layer {
    /// Single strong keep-alive for the backend mount, its fs, and its root.
    pub(super) mount: Arc<Mount>,
    /// Layer root dentry; explicit so subdirectory-anchored layers stay correct.
    pub(super) root_dentry: Arc<Dentry>,
    pub(super) fsid: u64,
    pub(super) container_dev_id: DeviceId,
}

// real.rs
pub(super) struct RealPath {
    mount: Weak<Mount>,
    dentry: Arc<Dentry>,
}

pub(super) enum RealObjectHandle {
    /// Inode-only, path-less handle.
    Inode(Arc<dyn Inode>),
    /// Dentry-anchored path handle; does NOT cache the inode.
    Path(RealPath),
}

pub(super) struct RealObject {
    layer_index: usize,
    fsid: u64,
    container_dev_id: DeviceId,
    handle: RealObjectHandle,
}
```

Decisions:

- `Layer` is the **single strong owner** of `Arc<Mount>`; `fs()` is derived
  from `mount.fs()`, and `root_path()` becomes infallible.
- `RealPath` stays `{ Weak<Mount>, Arc<Dentry> }`; it is a borrowed,
  re-resolvable dentry carrier, not a keep-alive owner.
- Do **not** introduce `Weak<Layer>` / `Arc<Layer>`.
- `RealObject` keeps `layer_index`, `fsid`, `container_dev_id`; these are not
  reachable from `Weak<Mount>` and are cheap immutable copies.
- The `Path` variant does not cache `real_inode`; `real_inode()` returns
  `Result<Arc<dyn Inode>>` and derives the inode through `RealPath`.
- Identity hot paths use `RealObjectKey` / `(fsid, real_ino)` instead of
  `Arc::ptr_eq` on `real_inode()`.

### 3.4 Persistent facts vs transient snapshot

Final target shapes:

```rust
// layer.rs (or inode/mod.rs)
/// Persistent real-object stack/composition owned by an OverlayInode.
pub(super) enum RealObjectStack {
    /// Upper-only object: no lower, never copy-ups, no Once.
    UpperOnly(RealObject),
    /// Lower-backed object: at least one lower; upper may be published once.
    LowerBacked {
        lowers: Vec<RealObject>,
        upper: Once<RealObject>,
    },
}

/// Transient, Clone-able snapshot used by readdir/projection.
#[derive(Clone, Debug)]
pub(super) struct RealObjectSnapshot {
    pub(super) upper: Option<RealObject>,
    pub(super) lowers: Vec<RealObject>,
}
```

Decisions:

- The persistent type is named **`RealObjectStack`**; the existing transient
  `RealObjectStack` is renamed to **`RealObjectSnapshot`**.
- `RealObjectStack` is non-`Clone` because `spin::Once<T>` is not `Clone`.
- `RealObjectSnapshot` remains `Clone` and keeps the existing `Option`/`Vec`
  shape.
- Do **not** merge the two types into one.
- Do **not** introduce a generic upper-slot abstraction over `Option`/`Once`.
- Boundary conversions are `OverlayInode::real_object_snapshot()` and
  `classify()` in `project_inode`; they are cost-equivalent to the clones that
  already happen today.
- The non-empty-lower invariant is enforced by constructor-only construction
  plus central validation. Do **not** add a `NonEmptyVec`/newtype now.

### 3.5 Inode enumization decision

- The two-branch persistent enum is **not worth doing as a standalone
  refactor**.
- It **is worth doing when bundled** with the `RealObjectHandle` enum and
  the `Layer` strong-mount work, which is the current plan.
- Before swapping the persistent representation, perform an accessor-only
  refactor on `OverlayInode` so call sites use `upper()`, `lowers()`,
  `is_upper_only()`, `has_lower()`, `visible_source()`, `lower_source()`,
  `publish_upper()`.
- The `match` on `RealObjectStack` stays inside accessors, not spread across
  call sites.
- Copy-up/rename publication paths need focused review: inode-cache rekey
  before `Once::call_once`, `UpperOnly` never enters copy-up, waiter re-check,
  and identity continuity.

### 3.6 Naming

Final naming set (entity-based naming):

| Old proposed name | Final name | Rationale |
|---|---|---|
| `RealObjectSource` | `RealObjectHandle` | It is a concrete handle to a real object, not an abstract source |
| `RealObjectFacts` | `RealObjectStack` | It is the persistent real-object stack/composition entity |
| existing transient `RealObjectStack` | `RealObjectSnapshot` | It is a transient cloneable snapshot of the stack |
| `Identity` variant | `Inode` | It carries an inode entity |
| `Pure` variant | `UpperOnly` | It names the stack shape: only an upper |
| `Impure` variant | `LowerBacked` | It names the stack shape: backed by at least one lower |

Constructors:

- `RealObject::identity_only(...)` → `RealObject::inode_only(...)` (or
  `from_inode(...)`).
- `RealObject::from_layer_path(...)` and `RealObject::child_hit(...)` keep
  their names.
- `OverlayInode::real_object_stack()` → `OverlayInode::real_object_snapshot()`.

## 4. Anti-Bloat Rules

Do:

- Keep `RealPath` weak.
- Keep exactly two new enum shapes: `RealObjectHandle` and
  `RealObjectStack`.
- Put accessors on the owning type (`Layer`, `RealObject`, `OverlayInode`,
  `RealObjectSnapshot`).
- Keep `RealObjectSnapshot` as the unchanged `Clone` transient type.
- Use `RealObjectKey` for identity hot paths.
- Accessorize first, then swap the persistent representation.

Do not:

- Do not introduce generics over `Option`/`Once`.
- Do not introduce `Arc<Layer>` / `Weak<Layer>`.
- Do not merge `RealObjectStack` and `RealObjectSnapshot`.
- Do not make `RealPath` strong or make `real_inode()` infallible.
- Do not cache `real_inode` in the `Path` variant.
- Do not add a `NonEmptyVec`/newtype now.
- Do not claim the enum removes all transient `lowers[0]` escapes; the
  transient snapshot legitimately remains an `Option`/`Vec` pair.
- Do not bundle unrelated features (metacopy, redirect_dir, index, readdir
  redesign) into this structural pass.

## 5. Proposal Revision Checklist

Update these parts of `structure-design-proposal.md`:

- §2 `OverlayFs`: field order per §3.2; use `mount.fs()` / `root_dentry`.
- §3 mount assembly: `LayerParts` becomes `(Arc<Mount>, Arc<Dentry>, DeviceId)`;
  `Layer::resolve_parts` captures `path.mount_node()` and `path.dentry()`.
- §4 `OverlayInode`: replace `lowers`/`upper` with `stack: RealObjectStack`;
  apply mixed field order; document accessor surface and `publish_upper()`.
- §5.1 `real.rs`: `RealObjectHandle` + `RealPath` definitions; no cached
  inode in `Path`.
- §5.2 `layer.rs`: strong `Arc<Mount>` `Layer`; `RealObjectStack` persistent
  vs `RealObjectSnapshot` transient.
- §6 lookup: `classify()` in `project_inode`; `(fsid, ino)` identity checks.
- §7 inode cache: `RealObjectKey` via `RealObject::key()`.
- §8 identity: `real_inode()` is `Result`; use `key()` where path is not needed.
- §9 readdir: use `RealObjectSnapshot` accessors.
- §10 copy-up: `publish_upper()`; rekey-before-`call_once`; `UpperOnly` never
  copy-ups.
- §11–13 mutation/data/xattr: accessor migration, no algorithm change.
- Add semantic glossary and legacy motivation.

## 6. Implementation Sequencing (if later implemented)

1. **Step 1 — `Layer` strong-mount**: independent PR. Replace
   `root_path.upgrade()?/expect(...)` with `root_dentry`/`mount.fs()`.
2. **Step 2 — `RealObjectHandle` enum**: independent PR after Step 1.
   `real_inode()` becomes `Result`; identity hot paths move to `(fsid, ino)`.
3. **Step 3 — Accessor-only refactor**: independent, low risk. Migrate
   persistent call sites to `OverlayInode` accessors without changing
   representation.
4. **Step 4 — `RealObjectStack` persistent enum**: atomic with accessor
   internals; ordinary call sites should already be migrated by Step 3.

Risk focus across steps: copy-up/rename publication, inode-cache rekey
ordering, readdir merged scans, remove/rename/link pure-upper vs lower-backed
branches, stale-upper detection, and identity continuity.

## 7. Consolidated Source Reports

The following temporary reports were consolidated into this handoff and then
removed from `designdoc/`:

- `exploration-20260824/01-legacy-and-semantics.md`
- `exploration-20260824/02-field-order.md`
- `exploration-20260824/03-inode-enum.md`
- `exploration-20260824/04-realobject-enum.md`
- `exploration-20260824/05-layer-realpath-realobject.md`
- `designer-20260825/designer-1-inode-two-branch.md`
- `designer-20260825/designer-2-realobject-paths.md`
- `designer-20260825b/designer-3-facts-stack-and-realobject-source.md`
- `designer-20260825c/designer-4-inode-enum-cost-benefit.md`
- `designer-20260825d/designer-5-overall-prudent-approach.md`
- `designer-20260825e/designer-6-naming.md`

The surviving proposal file to be modified is:

- `kernel/core/src/fs/fs_impls/overlayfs/.agents/designdoc/structure-design-proposal.md`
