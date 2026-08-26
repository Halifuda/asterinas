<!-- SPDX-License-Identifier: MPL-2.0 -->

# Introducing a structural refactor design for `overlayfs`

## Motivation

The legacy overlayfs is a single-file implementation with poor extensibility and maintainability, concurrency issues, and incomplete basic functionality. Starting from the domain concepts of overlayfs, this issue proposes a reimplementation design and clarifies the overlayfs ownership and reference model.

### Main Shortcomings of the Legacy Implementation

- **Single-file monolith**: all functionality is packed into one `legacy_fs.rs` file, making it hard for readers to locate behavior by type and hard to extend.
- **Unsafe identity and concurrency**: accessing the same underlying file multiple times produces multiple different overlay inodes, so state cannot be shared; concurrent writers may repeatedly perform the same operation.
- **Work directory behavior does not match Linux**: copy-up does not use an isolated work directory for atomic replacement, so a crash may leave half-finished results; multiple overlays may share the same work directory and interfere with each other.
- **Insufficient functionality and correctness**: capabilities such as read-only overlays, cross-directory rename, sync, superblock information, and permission checking are missing; directory enumeration is inefficient for large directories and its read position is unstable, and externally visible file identities (dev/ino) are also unstable.

### Goals of the Reimplementation

This issue proposes reimplementing overlayfs so that the code is anchored by core types and organized around domain concepts, while fixing the legacy problems in identity reuse, concurrency, work directory handling, and enumeration stability. The design emphasizes a clear ownership and reference model so that every mechanism has an explicit home and a clear reading path.

## Design
### 1. Overall Structure

The top level of the code consists of the following entries:

- `mod.rs`: module declaration and `init`.
- `fs_type.rs`: `OverlayFsType`, the VFS registration type.
- `fs/`: the mount module, representing the state owned by one mount and the construction flow.
- `inode/`: the logical object module, representing the `OverlayInode` exposed by overlayfs to the VFS and its family of behaviors.
- `real.rs`: the real object reference model.
- `layer.rs`: the layer model and transient real object stack.

In terms of dependency direction, `fs/` and `inode/` both depend on `real.rs` and `layer.rs`; `fs/` also depends on the cache and whiteout types in `inode/`. These two foundational files are at the bottom.

### 2. Mount Object `OverlayFs`

`fs/mod.rs` defines `OverlayFs` as all the state owned by one mount:

```rust
pub struct OverlayFs {
    /// Layer stack: composed of at most one writable layer (upper) and one or more read-only layers (lower).
    layer_stack: LayerStack,
    /// Runtime read-only mount policy.
    policy: MountPolicy,
    /// External dev/ino identity translation policy.
    identity: IdentityPolicy,
    /// Exclusive ownership of upper/workdir; `Some` exists only for writable mounts.
    upper_workdir_pair: Option<UpperWorkdirInuse>,
    /// Single-slot shared cache for whiteout.
    whiteout_cache: Mutex<WhiteoutCache>,
    /// inode identity reuse cache.
    inodes: InodeCache,
    /// VFS event statistics.
    fs_event_stats: FsEventSubscriberStats,
    /// Weak self-reference used to construct the root inode.
    self_weak: Weak<OverlayFs>,
    /// The overlay's `AnonDeviceId` RAII guard.
    _anon_device_id: AnonDeviceId,
}
```

Here we briefly explain the corresponding overlay concepts for several fields:

- `layer_stack`: at mount time, overlayfs assembles multiple underlying directories into a layer stack in order and maintains an upper-first merged view; see §5 for the layer model.
- `policy`: stores runtime mount policies such as read-only mode and permission settings, and determines whether writes and copy-up are allowed.
- `identity`: holds the dev/ino translation policy used to compute each overlay object's visible identity; see §10.
- `upper_workdir_pair`: holds the exclusive upper/workdir claim and the workdir staging resources for writable mounts; see §3 and §9.
- `whiteout_cache`: a mount-level shared cache related to the whiteout mechanism that hides lower names; here we only note that it serves directory namespace changes, see §12.
- `inodes`: the inode identity reuse cache; see §8.

The field order follows “core immutable state → synchronized state → cache/resource/weak reference”, making it easy to read the essence of the object from top to bottom.

### 3. Mount Construction Flow `fs/mount/`

`fs/mount/` is a one-time construction flow:

1. `options.rs`: parses `MountOptions`.
2. `layer_parts.rs`: parses the upper/lower/workdir root paths, validates them, and assembles the `LayerStack`. Here it directly parses `Arc<Mount>` and `Arc<Dentry>` as inputs to `Layer`. Validation includes: layer roots must not be the same, must not be ancestors/descendants of one another, and must not cross mount boundaries; the workdir must not conflict with lower roots.
3. `inuse.rs`: exclusively claims upper/workdir and prepares the workdir. Exclusiveness is necessary because when multiple overlays share the same upper/workdir, they overwrite each other's whiteouts, temporary objects, and directory state, corrupting each other's data.
4. `capabilities.rs`: measures upper capabilities such as `d_type` support and overlay private xattr support, and accordingly decides whiteout representation and other mount policies.
5. `mod.rs`: orchestrates the above steps.

### 4. Logical Object `OverlayInode`

`OverlayInode` is the carrier of the logical object that overlayfs exposes to the VFS:

```rust
pub struct OverlayInode {
    /// The overlay mount this inode belongs to.
    fs: Weak<OverlayFs>,
    /// Immutable lower real object stack, topmost first; lowers are real objects in read-only layers.
    lowers: Vec<RealObject>,
    /// Upper real object; upper is the real object in the writable layer, published at most once during copy-up.
    upper: Once<RealObject>,
    /// Precomputed externally visible `st_dev` / `st_ino`.
    object_id: ObjectId,
    /// Per-inode unique transaction lock; for directories it carries `ReaddirIndex` (the stable enumeration index of a merged directory).
    lock: Mutex<Option<ReaddirIndex>>,
    /// Logical overlay parent directory; it is also the publication parent directory of copy-up.
    parent: RwMutex<Weak<OverlayInode>>,
    /// Copy-up state: Done or Outstanding(CopyUpTarget).
    copyup: Mutex<CopyUpState>,
    /// Per-inode extension state provided by the VFS.
    extension: Extension,
}
```

Field explanations:

- `lowers` / `upper`: lowers are real objects in read-only layers, and upper is the real object in the writable layer. When same-named directories exist in multiple layers, overlayfs forms a merged directory (see §11). `upper: Once<RealObject>` means copy-up is a one-way, at-most-once publication from lower to upper; the read path is lock-free, and publication writes it atomically.
- `parent`: the logical overlay parent directory and the copy-up publication parent directory.
- `copyup`: stores the copy-up state; detailed semantics are in §9.
- `lock`: the directory transaction lock and readdir index domain; `ReaddirIndex` is the stable enumeration index of a merged directory, see §11. For non-directories it serves as a pure serialization token.
- `object_id`: the precomputed external dev/ino, computed by `IdentityPolicy`, see §10.

### 5. Layer Model `layer.rs`

overlayfs stacks several underlying directories into one visible namespace: at most one writable directory acts as the **upper**, the remaining read-only directories act as **lower**, and lookup first checks upper, then searches downward in top-to-bottom order through the lowers. `Layer` represents one of these layers:

```rust
pub struct Layer {
    /// Strongly holds the underlying mount; `Mount` owns `Arc<dyn FileSystem>` and the root dentry.
    mount: Arc<Mount>,
    /// Layer root dentry; upperdir/lowerdir may point to a subdirectory under a mount, so it must be stored explicitly.
    root_dentry: Arc<Dentry>,
    /// fs id dynamically assigned per mount according to the layer stack order.
    fsid: u64,
    /// Device id of the underlying fs.
    container_dev_id: DeviceId,
}
```

`Layer` is the overlay's **single strong owner** of an underlying mount/fs: only by strongly holding the mount can it guarantee that the underlying file system will not be unmounted first while the overlay mount is alive. `RealPath` (see §6) uses a weak mount and is a transient, re-resolvable anchor, so it cannot bear the keep-alive responsibility.

`root_dentry` is not necessarily the same as `mount.root_dentry()`. `lowerdir` / `upperdir` may point to a subdirectory under a mount and need not equal that mount's root; therefore `Layer` must explicitly store the layer root dentry and cannot assume the layer root is always the mount root.

overlayfs assumes that upper/lower are not modified directly, bypassing the overlay, while it is mounted; directly modifying an underlying layer is undefined behavior.

`LayerStack` represents the ordered collection of upper/lower layers, where `lowers` has at least one entry at mount time:

```rust
pub struct LayerStack {
    upper: Option<Layer>,
    lowers: Vec<Layer>,
}
```

The transient real object stack serves as the scanning carrier for lookup/readdir (see §7):

```rust
pub struct RealObjectStack {
    upper: Option<RealObject>,
    lowers: Vec<RealObject>,
}
```

### 6. Real Object Reference `real.rs`

`RealPath` is a weak-mount dentry-anchored carrier:

```rust
pub struct RealPath {
    mount: Weak<Mount>,
    dentry: Arc<Dentry>,
}

impl RealPath {
    /// Infallible: the stored `Arc<Dentry>` keeps the inode alive.
    pub fn inode(&self) -> &Arc<dyn Inode> {
        self.dentry.inode()
    }

    pub fn upgrade(&self) -> Result<Path> {
        Path::new(self.mount.upgrade()?, self.dentry.clone())
    }
}
```

All real objects come from already-resolved underlying paths, so `RealObject` is always path-backed:

```rust
pub struct RealObject {
    /// Index of the layer it belongs to.
    layer_index: usize,
    /// Real path anchored by a dentry.
    path: RealPath,
    /// fs id of the layer it belongs to.
    fsid: u64,
    /// Device id of the underlying fs.
    container_dev_id: DeviceId,
}
```

`RealObjectKey` is a value type that identifies a real object's identity inside overlayfs, composed of `fsid` and the real inode number, and is used as the key of the inode cache:

```rust
pub struct RealObjectKey {
    fsid: u64,
    real_ino: u64,
}
```

### 7. Name Resolution `inode/lookup.rs`

The lookup rule is:

- **upper-first**: first check upper, then search downward through lower layers in top-to-bottom order.
- **first-wins**: the merged result keeps only the first occurrence of a name, with upper layers taking precedence.
- **whiteout stops**: a whiteout in some layer hides the same-named object further down, and scanning for that name stops.
- **opaque directory stops**: when a directory in some layer carries the opaque marker, same-named directories further down stop participating in merging.
- **same-named non-directory stops**: when a name is a non-directory in a higher layer, it cannot merge with a lower-layer directory, and scanning continues downward no more.

A whiteout is a hiding marker in some layer, used to hide a same-named object in lower layers; an opaque directory is a marker on a directory in some layer, used to prevent lower-layer directories from continuing to participate in merging. These markers are usually written by upper, but when they appear in any layer they affect lower layers. Detailed representation and publication are in §12.

`Lookup` represents the result of one name resolution: `Positive` is the hit logical inode, and `Negative` is a miss or a name hidden by a marker.

```rust
pub enum Lookup {
    Positive(Arc<OverlayInode>),
    Negative(NegativeLookup),
}

pub enum NegativeLookup {
    Absent,
    HiddenByWhiteout,
    HiddenByOpaque,
}
```

Flow:

1. `lookup_in_layers` scans each layer and constructs a `RealObjectStack`;
2. on a positive hit, it constructs according to the hit source and initializes `OverlayInode` with `(parent, name)`: the new inode's `parent` and `CopyUpState` are initialized during construction; upper-backed objects are `Done`, lower-backed objects are `Outstanding`;
3. on a miss or a hit on whiteout/opaque, it constructs a `NegativeLookup`;
4. it returns `Lookup`.

The publication target `(parent, name)` is recorded during construction.

### 8. Inode Identity Reuse `inode/inode_cache.rs`

```rust
/// Weak keyed map from the current visible source identity to the shared
/// logical overlay inode. The cache does not keep overlay inodes alive.
struct InodeCache {
    entries: HashMap<RealObjectKey, Weak<OverlayInode>>,
}

impl InodeCache {
    /// Returns the existing live overlay inode for the same real object,
    /// or constructs and publishes a new one.
    fn get_or_create(
        &mut self,
        key: RealObjectKey,
        create: impl FnOnce() -> Arc<OverlayInode>,
    ) -> Arc<OverlayInode>;

    /// Migrates a logical inode's cached identity from its pre-copy-up lower
    /// key to the new upper key after copy-up publishes the upper object.
    /// The `OverlayInode` object itself stays the same.
    fn rekey(&mut self, old_lower: RealObjectKey, new_upper: RealObjectKey);
}
```

The same real object must yield the same `OverlayInode` through any name resolution; otherwise real-object composition, append write locks, and copy-up coordination would split. `RealObjectKey` is computed from the current visible source (the topmost hit real object). The complete lower stack of a merged directory is stored inside `OverlayInode`, so the key only identifies the current visible source.

### 9. Write Path and Copy-up `inode/copyup/`

```rust
/// Copy-up state of one overlay inode.
enum CopyUpState {
    /// Already upper-backed; copy-up has completed.
    Done,
    /// Still lower-backed; copy-up has not completed.
    Outstanding(CopyUpTarget),
}

struct CopyUpTarget {
    /// The directory-entry name used for publication in the parent.
    name: String,
    /// True when the physical rename succeeded but the overlay's internal
    /// state update failed, so the next copy-up must verify the upper target.
    need_repair: bool,
}
```

The state definitions describe where the object is published; the flow below shows how that target is read from the locked state and used to stage and publish the upper object.

```rust
/// Acquires the per-object copy-up mutex. The per-object mutex serializes
/// concurrent copy-up attempts on the same object: after the winner completes,
/// later callers see `upper` already set and return without repeating copy-up.
/// Callers check `upper` first; the guard also exposes the recorded
/// publication target.
fn lock_copyup(inode: &Arc<OverlayInode>) -> Result<MutexGuard<CopyUpState>>;

/// Stage data/metadata/xattr in the workdir.
fn stage_in_workdir(inode: &Arc<OverlayInode>, target: &CopyUpTarget) -> Result<WorkdirTemp>;

/// Atomically rename a staged workdir object into the upper parent.
fn publish_by_rename(
    inode: &Arc<OverlayInode>,
    target: &CopyUpTarget,
    staged: WorkdirTemp,
) -> Result<()>;

fn copy_up(inode: &Arc<OverlayInode>) -> Result<()> {
    if inode.upper.get().is_some() {
        return Ok(());
    }

    let guard = lock_copyup(inode)?;
    let target = match &*guard {
        CopyUpState::Done => return Ok(()),
        CopyUpState::Outstanding(target) => target,
    };

    let staged = stage_in_workdir(inode, target)?;
    publish_by_rename(inode, target, staged)
}
```

The ancestor chain in the lock-based promotion trigger ensures a child is published only after its parent directory already exists in upper. Workdir staging prevents exposing half-finished objects: a crash before publication leaves only a private workdir object, not a visible upper entry.

When the physical rename has already succeeded but the overlay's internal state update fails, `need_repair` is set. The next copy-up first verifies whether the upper target is consistent with lower, and then either continues reusing it or reports an error.

### 10. User-Visible Identity `inode/identity.rs`

overlayfs needs a separate identity module because after merging multiple underlying file systems, the underlying `st_dev` / `st_ino` cannot simply be exposed:

- different underlying file systems may have conflicting `st_dev` / `st_ino`, and passing them through directly would make user space misjudge distinct objects as the same;
- overlayfs needs to choose among passthrough (directly passing through the underlying dev/ino), xino encoding (re-encoding the underlying ino to avoid conflicts), or fallback (falling back to identities allocated by overlayfs) based on underlying capabilities and mount configuration, in order to provide stable, non-conflicting external identities;
- identity must remain stable before and after copy-up: copy-up changes the physical source, but the user still sees the same overlay object;
- `ObjectId` and `LowerIdOrigin` are the core entities of this module.

`IdentityPolicy` maps a real object to the overlay's externally visible `st_dev` / `st_ino`:

- `ObjectId`: the external dev/ino of an overlay object;
- `LowerIdOrigin`: the persisted identity record of the lower source before copy-up.

### 11. Directory Enumeration `inode/readdir.rs`

When same-named directories exist in both upper and lower, overlayfs presents a **merged directory**: the visible names are the union of upper and the lowers, upper takes precedence, and metadata is based on upper.

The goal of readdir is to provide a stable, resumable enumeration order and avoid a full rescan on every getdents (the system call that reads directory entries).

Enumeration of a merged directory is divided into the following phases:

1. **Index construction on demand**: when a directory is enumerated for the first time, each real layer is scanned in upper-first, lower top-to-bottom order; each name is kept only at its first occurrence; whiteouts, opaque directories, and same-named non-directories suppress or terminate lower names from continuing to merge.
2. **Stable cookie sequence**: `.` and `..` use fixed cookies, and every other visible name obtains a cookie in order; all cookies are monotonically increasing and never reused; callers use a cookie as the offset (read position) to resume reading from the previous position.
3. **Rebuild and invalidation**: after a namespace change, if it cannot be proven that the existing order still holds, the directory index is marked as needing rebuild and is rescanned on the next enumeration. create/link may insert a name whose true position cannot be proven without a full scan; rename reorders names; whiteout/opaque changes alter visibility; these cases may trigger a rebuild. Removing a name may retain its cookie position (tombstone), preventing already-exposed offsets (read positions) from being renumbered.

### 12. Namespace Changes `inode/dir/`

`inode/dir/` contains all operations that change the namespace of a parent directory:

- `create.rs` / `link.rs` / `remove.rs` / `rename.rs`
- `whiteout.rs`: representation, publication, and cleanup of whiteouts

When deleting a name visible from lower, lower cannot be modified; instead a whiteout hiding marker is published in upper. The physical representation of a whiteout is chosen according to upper capabilities: it can be a char device `0:0`, or a zero-size file with the `trusted.overlay.whiteout` xattr.

`WhiteoutCache` is a mount-level, single-slot reuse pool that caches private whiteout temporary objects located in workdir, for later publication as whiteouts in upper. Its principle is that the same workdir whiteout can be published to multiple upper directories/names via hard links, so remove/rename do not need to create a whiteout temporary object every time.

Usage:

- both the remove and rename lower-backed paths publish whiteouts through the same `publish_whiteout` entry.
- only when the name does not yet exist in upper and the underlying layer supports hard-link sharing is the workdir whiteout stored back into the cache after successful publication.
- if the target already has an upper object, publication needs rename-over, or hard-link sharing fails, the whiteout temporary object is consumed and not returned to the cache. Hard-link sharing may fail due to `EMLINK` (link count limit reached) or `EOPNOTSUPP` (backend does not support the operation).
- once `can_share_by_link` is lowered to false due to a hard-link failure, it remains disabled; subsequent whiteouts are published by consuming new temporary objects.

Cached temporary objects always live in workdir and are private objects; leftover workdir entries are cleaned up the next time the workdir is prepared during mount.

### 13. Permissions, Attributes, xattr, Data

- `inode/permission.rs`: two-stage permission checking—first perform overlay-local permission checks and, when needed, perform copy-up on demand, then check the underlying real object's permissions; copy-up is an action between the two stages, not a third permission check.
- `inode/metadata.rs`: attribute writes.
- `inode/xattr.rs`: xattr operations and overlay private xattrs.
- `inode/data.rs`: data read/write forwarding; reads of lower use `O_NOATIME`, and `O_APPEND` is serialized under the per-inode transaction lock.

### 14. File Structure

```text
overlayfs/
├── mod.rs                    — module declaration + init
├── fs_type.rs                — OverlayFsType: VFS registration type
├── layer.rs                  — Layer / LayerStack / RealObjectStack: underlying directory layer model
├── real.rs                   — RealObject / RealPath / RealObjectKey
│
├── fs/                       — mount module
│   ├── mod.rs                — OverlayFs + FileSystem implementation
│   ├── policy.rs             — MountPolicy
│   └── mount/                — construction flow
│       ├── mod.rs            — construction flow orchestration
│       ├── options.rs        — MountOptions parsing
│       ├── layer_parts.rs    — layer root parsing and overlap/workdir validation
│       ├── inuse.rs          — exclusive claim of upper/workdir
│       └── capabilities.rs   — upper capability measurement
│
└── inode/                    — logical objects
    ├── mod.rs                — OverlayInode + Inode/FileOps implementation
    ├── inode_cache.rs        — reuse of the same underlying object as the same OverlayInode
    ├── lookup.rs             — layer-ordered name resolution
    ├── identity.rs           — external dev/ino identity translation
    ├── readdir.rs            — stable enumeration of merged directories
    ├── copyup/
    │   ├── mod.rs            — copy-up arbitration/preparation/publication
    │   └── workdir.rs        — workdir temporary objects
    ├── dir/
    │   ├── mod.rs            — shared namespace change flow
    │   ├── create.rs / link.rs / remove.rs / rename.rs
    │   └── whiteout.rs       — whiteout representation/publication/cleanup
    ├── data.rs
    ├── permission.rs
    ├── metadata.rs
    └── xattr.rs
```
