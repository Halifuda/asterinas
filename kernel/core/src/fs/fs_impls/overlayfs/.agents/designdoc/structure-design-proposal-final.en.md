<!-- SPDX-License-Identifier: MPL-2.0 -->

# Introducing the Structural Reimplementation Design of `overlayfs`

## Motivation

overlayfs is a stackable union filesystem: it merges an **optional** writable directory (upper) with several read-only directories (lower) top-down into a single visible directory tree; on writes, it copies modified objects into upper and never modifies lower in place.

### Main Shortcomings of the Legacy Implementation

- **Single-file monolith**: all functionality is crammed into a single `legacy_fs.rs` file, making it hard for readers to locate behavior by type and hard to extend.
- **Unsafe identity and concurrency**: when the same underlying file is accessed multiple times, multiple distinct overlay inodes are generated and state cannot be shared; concurrent writers may repeat the same operation.
- **Work-directory behavior that does not match Linux**: copy-up does not use an isolated work directory for atomic replacement, so a crash may leave half-finished artifacts behind; multiple overlays may share the same work directory and interfere with each other.
- **Insufficient functionality and correctness**: capabilities such as read-only overlays, cross-directory rename, sync, superblock information, and permission checks are missing; directory enumeration is inefficient on large directories with unstable resume positions, and the external file identity (dev/ino) is unstable as well.

### Goals of the Reimplementation

Most of the legacy shortcomings stem from a lack of model organization. This design reimplements overlayfs starting from its domain concepts, letting the code anchor on core types and be organized by domain concepts, and fixes the aforementioned problems such as identity reuse, concurrency, work directory, and enumeration stability; the ownership and reference model of each mechanism thereby becomes explicit as well.

## Design
### 1. Overall Structure

The top level of the code consists of the following entry points:

- `mod.rs`: module declarations and `init`.
- `fs_type.rs`: `OverlayFsType`, the VFS registration type.
- `fs/`: the mount module, representing the state owned by one mount and its construction flow.
- `inode/`: the logical-object module, representing the `OverlayInode` that overlay exposes to VFS and its family of behaviors.
- `real.rs`: the real-object reference model.
- `layer.rs`: the layer model; each layer holds a private mount view of the underlying mount that belongs solely to itself, and the layer root is carried by the view.

In terms of dependency direction, both `fs/` and `inode/` depend on `real.rs` and `layer.rs`; `fs/` additionally depends on the cache and whiteout types in `inode/`. These two foundational files sit at the very bottom.

### 2. The Mount Object `OverlayFs`

`fs/mod.rs` defines `OverlayFs`, which serves as all the state owned by one mount:

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
    /// Single-slot shared cache for whiteouts.
    whiteout_cache: Mutex<WhiteoutCache>,
    /// Inode identity reuse cache.
    inodes: InodeCache,
    /// VFS event statistics.
    fs_event_stats: FsEventSubscriberStats,
    /// The overlay's `AnonDeviceId` RAII guard.
    _anon_device_id: AnonDeviceId,
    /// Weak self-reference used to construct the root inode.
    self_weak: Weak<OverlayFs>,
}
```

- `layer_stack`: at mount time, overlayfs assembles multiple underlying directories in order into a layer stack and maintains an upper-first merged view; see #4, the layer model, for details.
- `policy`: holds the runtime mount policy, such as read-only mode and permission settings, and decides whether writes and copy-up are allowed.
- `identity`: holds the dev/ino identity translation policy, used to compute the externally visible identity of each overlay object; see #10 for details.
- `upper_workdir_pair`: holds the exclusive claim on upper/workdir of a writable mount together with the workdir staging resources. The workdir is a temporary directory on the filesystem that hosts upper; objects for copy-up and whiteout are staged in it first and then published to their final positions by atomic rename. The exclusive claim on and preparation of the workdir are covered in #3 (construction flow step 3), the publication of copy-up in #9, and the publication and cleanup of whiteouts in #12.
- `whiteout_cache`: the mount-level shared cache associated with the whiteout mechanism that hides lower names; here we only state for now that it serves directory namespace changes, see #12 for details.
- `inodes`: the inode identity reuse cache; see #8 for details.

### 3. The Mount Construction Flow `fs/mount/`

`fs/mount/` is a one-shot construction flow:

1. `options.rs`: parses `MountOptions`.
2. `layer_parts.rs`: parses the upper/lower/workdir root paths, validates them, and assembles the `LayerStack`. Validation covers: layer roots must not be identical, must not be ancestors/descendants of one another, and must not cross mount boundaries; the workdir must not conflict with the lower roots. During assembly, a private mount view rooted at its resolved path is also constructed for each layer and for the workdir, reusing VFS's existing mount-clone primitives.
3. `inuse.rs`: exclusively claims upper/workdir and prepares the workdir, preventing multiple overlays that share the same upper/workdir from overwriting each other's whiteouts, temporary objects, and directory state and thereby corrupting each other's data.
4. `capabilities.rs`: measures upper capabilities, such as `d_type` support and overlay private xattr support, and decides the whiteout representation and other mount policies accordingly.
5. `mod.rs`: orchestrates the steps above.

### 4. The Layer Model `layer.rs`

overlayfs stacks several underlying directories into one visible namespace: at most one writable directory serves as **upper**, the remaining read-only directories serve as **lower**, and the layers are arranged in the fixed order given at mount time, with upper at the very front. `Layer` represents one of these layers:

```rust
pub struct Layer {
    /// Strong hold on this layer's private mount view; the view's root dentry is the layer root.
    mount: Arc<Mount>,
    /// The fs id dynamically assigned at each mount according to the layer stack order.
    fsid: u64,
    /// The device id of the underlying fs.
    container_dev_id: DeviceId,
}
```

`Layer` is the sole strong holder of this layer's mount view, guaranteeing that the underlying filesystem and the layer root are not reclaimed prematurely while the overlay is alive. The layer root need not be the root of the underlying mount, so during assembly each layer obtains a private cloned view rooted at its resolved layer path; the private view registers with no mount namespace.

overlay assumes that upper/lower will not be modified directly, bypassing the overlay, while the mount is up (directly modifying the underlying layer is undefined behavior; see `Documentation/filesystems/overlayfs.rst`: "directly modifying underlying filesystems could result in undefined behavior", and lower is expected to remain read-only).

`LayerStack` represents the ordered collection of upper/lower layers, in which `lowers` has at least one entry at mount time:

```rust
pub struct LayerStack {
    upper: Option<Layer>,
    lowers: Vec<Layer>,
}
```

### 5. The Logical Object `OverlayInode`

`OverlayInode` is the carrier of the logical objects that overlay exposes to VFS:

```rust
pub struct OverlayInode {
    /// The overlay mount this inode belongs to.
    fs: Weak<OverlayFs>,
    /// The immutable stack of lower real objects, topmost first; lower objects are real objects in the read-only layers.
    lowers: Vec<RealObject>,
    /// The upper real object; upper is a real object in the writable layer, published at most once at copy-up.
    upper: Once<RealObject>,
    /// The precomputed external `st_dev` / `st_ino`.
    object_id: ObjectId,
    /// Per-inode unique transaction lock; for directories it carries `ReaddirIndex` (the stable enumeration index of a merged directory).
    lock: Mutex<Option<ReaddirIndex>>,
    /// The recorded parent: the logical overlay parent directory; also the publication parent of copy-up.
    recorded_parent: RwMutex<Weak<OverlayInode>>,
    /// The arbiter and publication name of copy-up; see #9 for details.
    copyup: Mutex<Option<String>>,
    /// The per-inode extended state provided by VFS.
    extension: Extension,
}
```

Field notes:

- `lowers` / `upper`: lower objects are real objects in the read-only layers, and upper is a real object in the writable layer. When a same-named directory exists in multiple layers, overlay forms a merged directory (see #11). `upper: Once<RealObject>` means copy-up is a one-way, at-most-once lower→upper publication; the read path takes no lock, and publication writes atomically.
- `object_id`: the precomputed external dev/ino, computed by `IdentityPolicy`, see #10.
- `lock`: the directory transaction lock and the readdir index field; `ReaddirIndex` is the stable enumeration index of a merged directory, see #11. For non-directories it carries no additional state and is merely used to serialize concurrent access to this object.
- `recorded_parent`: the logical overlay parent directory and the copy-up publication parent. The "recorded parent" — it is the record written down at first binding, not a fact re-derived at each access.
  - **Binding rule**: the publication coordinates `(recorded_parent, name)` are established once and for all by the first forward resolution at construction time; later lookups that hit the cache never write back; a single update happens only when a cross-directory rename succeeds.
  - **Multi-link trade-off**: multiple aliases of the same underlying object follow first-seen-wins, each converging to its own upper publication; alias relinking (the index family) is explicitly out of scope.
  - **The publication coordinates record the name at first binding**, not the name traversed when some copy-up happens to be triggered; the two can differ. When they differ, the physical copy lands at the coordinate position, the canonical object turns upper-backed and keeps serving its existing handles, and the other aliases are judged stale-upper in their own subsequent resolutions and rebuilt as independent lower-backed instances — the source data they see stays at the moment before copy-up, and each catches up only after going through copy-up again.
  - **Not derived from the underlying dentry**: the evolution of the overlay namespace can run ahead of the physical form (the redirect-style "change the logical name first, then decide whether to migrate", as well as transitional moments such as whiteout shadowing and identity-cache rekeying); what this field carries is "where overlay thinks it is", not "where it physically is at this moment".
- `copyup`: the arbiter and publication name of copy-up; see #9 for the detailed semantics.

#### The Trade-offs Around `recorded_parent`

**Use**: copy-up relies on it to copy up level by level along the parent chain, until the parent directory exists in upper; a prepared copy needs a definite landing place. readdir relies on it, so that `".."` must give the external identity of the parent directory, while the parent directory is another overlay object that the current interface does not pass in.

**Problem with the current formulation**: first-binding coordinates ≠ the triggering context — when a write goes through `/b/y` while the coordinates record `/a/x`, the physical copy lands at `/a/x`, and the triggering path then rebuilds its own instance according to the alias-splitting rule.

**Alternative on the VFS side**: following Linux's practice of letting the dcache carry the parent-child relationship, `&Dentry` (or its `NameAndParent`) is passed in as call-time context through the Inode trait, so that the publication coordinates of each operation are exactly "the parent traversed this time", and the persistent field is deleted.

### 6. Real-Object References `real.rs`

`RealObject` is a reference to a real object in one layer: `layer_index` identifies the layer it resides
in, and the dentry anchors the concrete entry of that layer; the identity of the layer (fsid / container
device id) is carried uniformly by the layer definition and is not copied per object.

```rust
pub struct RealObject {
    /// The ordinal of the layer this object resides in.
    layer_index: usize,
    /// The real entry anchored by the dentry.
    dentry: Arc<Dentry>,
}
```

A complete `Path` can be rebuilt on demand via the mount view of the layer it resides in; the anchor's validity follows the overlay's lifetime — as long as the logical object is reachable, the view and dentry it references will not be reclaimed.

`RealObjectKey` is the value type used inside overlay to identify the identity of a real object, composed of `fsid` and the real inode number, and serves as the key of the inode cache:

```rust
pub struct RealObjectKey {
    fsid: u64,
    real_ino: u64,
}
```

### 7. Name Resolution `inode/lookup.rs`

Name resolution scans in the fixed order of the layers: same-named objects merge into one logical object; same-named directory objects across the layers merge into one object, while for non-directories the topmost one prevails and is returned; a layer's whiteout and opaque markers truncate the contributions below them at their own level.

`Lookup` represents the result of one name resolution: `Positive` is the logical inode that was hit, and `Negative` is a miss or a shadowed name.

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

The three negative variants uniformly present `ENOENT` externally; the differences matter only on the fs-internal decision surface:

- **Absent**: the name exists in no layer; for upper it is just an ordinary miss, and creation takes upper's regular new-creation path.
- **HiddenByWhiteout**: a whiteout is a **name-level shadow** — an independent hidden object in some layer (such as a char device `0:0` or a marked zero-length file) that blocks same-named entries in the layers below it. Example: upper has a whiteout named `foo` ⇒ the name is completely invisible, and enumeration skips it. For upper: creation must go through over-whiteout preparation and replacement, not bare creation; a deletion request issued against it only gets ENOENT — the shadow remains in place and is not touched; rename recognizes such targets as whiteout targets and flips the Replace/Exchange choice.
- **HiddenByOpaque**: opaque is a **directory-level marker** stamped on a real directory of some layer — it qualifies the whole real directory rather than some name object; when a directory in the upper layer carries this marker, every same-named contribution from lower is cut off. Example: an upper directory marked opaque ⇒ the visible set is exactly its own entries. For upper: it merges with `Absent` into plain-create; during enumeration the lower directory exits the merge as a whole.

These markers are usually written by upper, but their appearance in any layer affects the layers further down. See #12 for the detailed representation and publication.

### 8. Inode Identity Reuse `inode/inode_cache.rs`

```rust
/// A weak-reference map keyed by the identity of the currently visible source, pointing to the shared logical overlay inode. The cache never keeps an overlay inode alive.
struct InodeCache {
    entries: HashMap<RealObjectKey, Weak<OverlayInode>>,
}

impl InodeCache {
    /// Returns the already-live overlay inode of the same real object if one exists; otherwise constructs and publishes a new inode.
    fn get_or_create(
        &mut self,
        key: RealObjectKey,
        create: impl FnOnce() -> Arc<OverlayInode>,
    ) -> Arc<OverlayInode>;

    /// After copy-up publishes the upper object, migrates the cached identity of the logical inode from the pre-copy-up lower key
    /// to the new upper key. The `OverlayInode` object itself stays unchanged.
    fn rekey(&mut self, old_lower: RealObjectKey, new_upper: RealObjectKey);
}
```

Resolving the same real object through any name resolution must yield the same `OverlayInode`, otherwise real-object composition, the append-write lock, and copy-up coordination would split apart. `RealObjectKey` is computed from the currently visible source (the topmost hit real object). The complete lower stack of a merged directory is kept inside `OverlayInode`, so the key only identifies the currently visible source.

### 9. The Write Path and copy-up `inode/copyup/`

When the user issues a write-type operation on an object whose content is still provided by lower, overlay first produces a writable upper copy and then atomically places it at the position of that name — this process is called copy-up. Everything produced during preparation is privately staged in the workdir: a crash before publication exposes no half-finished artifacts to upper.

The arbitration and publication name of copy-up are carried by a mutex: an object that is still lower-backed holds `Some(name)`, meaning it is pending publication to `(recorded_parent, name)`; once publication succeeds it is set to `None` and retires permanently — from then on the object never has any copy-up transaction again. The single source of truth for "whether it has been published" is whether `upper` is set, and the two stay consistent by "converging together at the completion of publication".

The action surface of copy-up falls on four methods of `impl OverlayInode`:

```rust
impl OverlayInode {
    /// Acquires the per-object copy-up mutex; the guard exposes the name pending publication.
    fn lock_copyup(&self) -> MutexGuard<'_, Option<String>>;

    /// Stages data/metadata/xattr in the workdir.
    fn stage_in_workdir(&self, name: &str) -> Result<WorkdirTemp>;

    /// Atomically renames the object staged in the workdir into the `name` position of the upper parent directory.
    fn publish_by_rename(&self, name: &str, staged: WorkdirTemp) -> Result<()>;

    fn copy_up(&self) -> Result<()> {
        if self.upper.get().is_some() {
            return Ok(());
        }
        let mut published = self.lock_copyup();
        let Some(name) = &*published else {
            return Ok(());            // no publication coordinates pending anymore
        };
        let staged = self.stage_in_workdir(name)?;
        self.publish_by_rename(name, staged)?;
        *published = None;            // the coordinates retire
        Ok(())
    }
}
```

For lock ordering only two fixed orders are introduced: first acquire the object's own copy-up lock, then perform ancestor promotion and preparation; `publish_by_rename`, while holding the copy-up lock, briefly acquires the parent directory's transaction lock to complete the physical rename and the semantic commit, and then releases it. Apart from this single "copy-up → directory transaction" locking edge, no new ordering is introduced.

copy-up proceeds level by level along the ancestor directory chain: before a child is published, its parent directory has already completed copy-up and exists in upper. Staging in the workdir avoids exposing half-finished artifacts: a crash before publication leaves behind only a private workdir object, never a visible upper entry.

The physical rename is the dividing line of copy-up. Any error before the rename is handled in exactly the same way: delete the temporary copy in the workdir and redo the whole process from the start, with the outside world seeing no trace.

After the rename succeeds only one finishing task remains: register the new identity in the inode cache and mark the logical object as upper-backed. Concurrency obeys one simple rule — when two tasks register the same file at the same time, the later one waits for the former to finish registering and then directly reuses the same logical object.

### 10. User-Visible Identity `inode/identity.rs`

overlayfs needs a separate identity module because after merging multiple underlying filesystems the underlying `st_dev` / `st_ino` cannot simply be exposed:

- Different underlying filesystems may have conflicting `st_dev` / `st_ino`, and passing them straight through would make user space mistake different objects for the same object;
- overlay needs to choose passthrough (directly passing through the underlying dev/ino), xino encoding (re-encoding the underlying ino to avoid conflicts), or fallback (falling back to identities assigned by overlay) according to the underlying capabilities and the mount configuration, so as to provide a stable and conflict-free external identity;
- the identity must remain stable across copy-up: copy-up changes the physical source, but what the user sees is still the same overlay object.

This module has two core entities:

- `ObjectId`: the external dev/ino of one overlay object;
- `LowerIdOrigin`: the persisted identity record of the lower source before copy-up.

The identity translation policy is carried by the mount-level `IdentityPolicy` and is fixed once and for all during assembly.

### 11. Directory Enumeration `inode/readdir.rs`

When a same-named directory exists in both upper and lower, overlay presents a **merged directory**: the visible names are the union of upper and each lower, upper takes precedence, and metadata follows upper.

The goal of readdir is to provide a stable, resumable enumeration order, avoiding a full rescan on every getdents (the system call that reads directory entries).

Enumeration of a merged directory is divided into the following phases:

1. **Build the index on demand**: the first time the directory is enumerated, scan the real layers in upper-first, lower top-to-bottom order; keep only the first occurrence of each name; whiteouts, opaque directories, and same-named non-directories suppress or terminate the continued merging of lower-layer names.
2. **Stable cookie sequence**: `.` and `..` use fixed cookies, and every other visible name obtains a cookie in order; all cookies increase monotonically and are never reused; the caller uses the cookie as the offset (resume position) to continue reading from the last position.

Rebuilding follows one criterion: a full rescan happens only when it cannot be proven that the existing cookie sequence still holds. The deletion of a single name falls to a tombstone and the resume positions are preserved; the appending of a new name is inserted in place if it can be proven to lie at the end of the existing sequence. Rename is the combination of a tombstone for the old name and an insertion for the new name: it merges in place when the insertion point can be proven to fall at the end of the sequence, and only otherwise degrades to a full rescan; offsets that have already been exposed are always guaranteed by tombstones not to be renumbered. Genuine rebuilds concentrate on sweeping visibility changes — the most typical case is the appearance or disappearance of the opaque marker, which adds or removes the lower contributions as a whole.

### 12. Namespace Changes `inode/dir/`

`inode/dir/` contains all operations that make changes to the parent directory namespace:

- `create.rs` / `link.rs` / `remove.rs` / `rename.rs`
- `whiteout.rs`: the representation, publication, and sweeping of whiteouts

Deleting a lower-visible name cannot modify lower; instead, a whiteout shadow is published in upper. The physical representation of a whiteout is chosen according to upper's capabilities: it can be a char device `0:0` or a zero-size file carrying the `trusted.overlay.whiteout` xattr.

`WhiteoutCache` is a mount-level, single-slot reuse pool that caches the private whiteout temporary objects located in the workdir for later publication as whiteouts in upper. Its principle is that the same workdir whiteout can be published via hard link under multiple upper directories/names, so remove/rename need not recreate the whiteout temporary object every time.

Usage:

- When deleting an object — no matter whether via unlink/rmdir or rename — as long as a whiteout needs to be left in upper to shadow the same-named lower object, the same publication path is taken: take the private temporary object from the workdir and atomically place it at the target name.
- Only when the name does not yet exist in upper and the underlying layer supports hard-link sharing is the workdir whiteout stored back into the cache after a successful publication.
- If the target already has an upper object and a rename-over publication is required, or hard-link sharing fails, the whiteout temporary object is consumed and does not backfill the cache. Hard-link sharing can fail due to `EMLINK` (the link count limit is reached) or `EOPNOTSUPP` (the backend does not support the operation).
- Once `can_share_by_link` has been downgraded to false by a hard-link failure it stays disabled; all subsequent whiteouts are published by consuming new temporary objects.

The cached temporary objects always reside in the workdir and are private; leftover workdir entries are cleaned up the next time a mount prepares the workdir.

### 13. Extended Attributes `inode/xattr.rs`

overlay needs to persist a batch of internal records on the real objects in upper like opaque and whiteout. If these names shared the same channel with user space, they could be forged or misread, so they are uniformly kept under overlay's **private prefix** — `trusted.overlay.` in trusted mode, `user.overlay.` in _userxattr_ mode. xattr handling splits names into only two classes, and the decision looks only at the prefix:

```rust
enum XattrClass {
    /// Begins with the private prefix.
    Private,
    /// Every other name (`user.plain.any`, `security.selinux`,
    /// `trusted.backup.notes`…): passed through as-is, overlay applies
    /// no interpretation of its own.
    Passthrough,
}

/// The default prefix. `CAP_SYS_ADMIN` privilege
/// is required, thus preventing non-root users from modifying.
const TRUSTED_OVERLAY_PREFIX: &str = "trusted.overlay.";

/// The prefix in userxattr mode. When the mounter doesn't have
/// `CAP_SYS_ADMIN`, use this mode to workaround. Cannot prevent
/// user-modification.
const USER_OVERLAY_PREFIX: &str = "user.overlay.";
```

An overlayfs has two paths to handle xattr:

```rust

impl OverlayInode {
    /// Private path: Overlayfs locally sets a xattr for its use,
    /// like `trusted.overlay.whiteout`.
    fn set_overlay_xattr(
        &self,
        name: XattrName,
        value_reader: &mut VmReader,
        flags: XattrSetFlags,
    ) -> Result<()> {
        // Pass the name to the underlying real object as-is.
        self.real_inode().set_xattr(name, value_reader, flags)
    }

    /// Passthrough path: Overlayfs handles syscalls from upper layers.
    /// It will check if the prefix of `name` matches its private prefix.
    /// If so, to avoid conflicts, it locally modifies the name.
    ///
    /// `set` is an example; both `get` and `set` may modify the name.
    pub(super) fn set_xattr_impl(
        &self,
        name: XattrName,
        value_reader: &mut VmReader,
        flags: XattrSetFlags,
    ) -> Result<()> {
        let mut used_name = String::from(name.full_name());
        if name.full_name().starts_with(selected_prefix) {
            // Example: "trusted.overlay.opaque" -> "trusted.overlay.overlay.opaque";
            // the midfix "overlay." goes right after the private prefix, so the rest
            // of the name (including any segments already present) shifts right.
            used_name.insert_str(selected_prefix.len(), "overlay.");
        }
        let used = XattrName::try_from_full_name(&used_name).ok_or(Errno::EINVAL)?;
        self.real_inode().set_xattr(used, value_reader, flags)
    }
}

```

In terms of the two paths above: only the passthrough path ever adds a segment, exactly one per layer it descends through, while the private path always uses unsegmented names. When N layers of nested overlay with the same prefix are stacked, each layer adds one segment, so the number of added segments in an on-disk name = the number of layers lying between the marker's owner and the underlying filesystem, and the markers of the separate layers are physically differentiated by segment count and invisible to one another.

When upper lacks xattr capability, the probe during mount construction measures this capability for real instead of assuming it; lacking capability does not mean read-only — creation, data reads and writes, and attribute changes depend on no marker and remain available as usual. Degradation follows **rather reject than err**: an operation whose correctness must depend on markers is refused outright, and a state that could be misread is never produced. The affected operations are accordingly divided into two families:

- **The rejected family**: creating a directory over a whiteout, and the replacement in a clear-empty exchange, must carry opaque; if it cannot be written in, `EOPNOTSUPP`.
- **The degraded family**: whiteout falls back to the char device `0:0` form; the origin record becomes a silent no-op, at the cost of degraded `st_ino` stability across copy-up; mount options that presuppose markers are switched off as a whole.

### 14. Permissions, Attributes, and Data

- `inode/permission.rs`: two-phase permission checking — first the overlay-local permission check, then a copy-up when needed, then the permission check against the underlying real object; copy-up is the action between the two phases, not a third phase of permission checking.
- `inode/metadata.rs`: attribute writes.
- `inode/data.rs`: forwarding of data reads and writes, with `O_NOATIME` when reading lower, and `O_APPEND` serialized under the per-inode transaction lock.

### 15. File Structure

```text
overlayfs/
├── mod.rs                    — module declarations + init
├── fs_type.rs                — OverlayFsType: the VFS registration type
├── layer.rs                  — Layer / LayerStack: the layer model of underlying directories
├── real.rs                   — RealObject / RealObjectKey
│
├── fs/                       — the mount module
│   ├── mod.rs                — OverlayFs + the FileSystem implementation
│   ├── policy.rs             — MountPolicy
│   └── mount/                — the construction flow
│       ├── mod.rs            — construction flow orchestration
│       ├── options.rs        — MountOptions parsing
│       ├── layer_parts.rs    — layer root resolution and overlap/workdir validation
│       ├── inuse.rs          — the exclusive claim on upper/workdir
│       └── capabilities.rs   — upper capability measurement
│
└── inode/                    — logical objects
    ├── mod.rs                — OverlayInode + the Inode/FileOps implementations
    ├── inode_cache.rs        — reuse from the same underlying object to the same OverlayInode
    ├── lookup.rs             — name resolution in layer order
    ├── identity.rs           — external dev/ino identity translation
    ├── readdir.rs            — stable enumeration of merged directories
    ├── copyup/
    │   ├── mod.rs            — copy-up arbitration/preparation/publication
    │   └── workdir.rs        — workdir temporary objects
    ├── dir/
    │   ├── mod.rs            — the shared namespace-change flow
    │   ├── create.rs / link.rs / remove.rs / rename.rs
    │   └── whiteout.rs       — whiteout representation/publication/sweeping
    ├── data.rs
    ├── permission.rs
    ├── metadata.rs
    └── xattr.rs
```

### 16. Gaps and no-goals

This chapter records the mechanism boundaries that have been identified but are not implemented for now. They **do not belong to the behavior scope this design commits to** — they are listed so that later trade-offs need not re-derive the starting point. There are three criteria for a mechanism to enter this chapter, and satisfying any one of them means "not implemented for now":

- **Blast radius of failures**: when the mechanism fails, what is affected is not a single operation but cross-object visible state (directory merging, hard-link sharing, data-source attribution), which would require crash/consistency arguments heavier than the current ones;
- **Trust prerequisites**: the mechanism requires premises such as "layer contents or markers cannot be forged", and this design has not yet brought untrusted layers into its threat model;
- **Platform dependence**: the mechanism waits for VFS/kernel capabilities that do not yet exist (the interface PR still pending merge, a handle-resolution interface that resolves a file handle back to an inode, and a permission-check mechanism that distinguishes caller requests from overlay-internal IO).

#### 1. The credential gap (internal IO executes with the mounter's credentials, not the caller's)

**Mechanism sketch**. overlay's internal IO — the operations overlay executes on the backend filesystem for the sake of its own mechanisms — cannot use the identity of the caller who triggered the operation; it includes the preparation and publication of copy-up, whiteout publication, and the faithful copying of xattrs (that is, copying the lower object's xattrs verbatim onto the upper copy at copy-up). The copy must be faithful: the party entitled to perform chown and to write `trusted.*` xattrs is the mounter, not the caller. Linux's solution is override creds: overlay captures the mounter's credentials at mount time, and internal IO executes temporarily with the mounter's credentials within that scope (`with_ovl_creds`, `fs/overlayfs/copy_up.c:1250`, `file.c:42`); before copy-up there is an additional LSM hook (`security_inode_copy_up`, `copy_up.c:732`) that obtains a transitional label for the new copy; at mount time `CAP_SYS_RESOURCE` is also deliberately dropped from the mounter's effective capability set (`super.c:1513`) to prevent internal writes from bypassing upper's quota. Example: an ordinary user appends one line to a root-owned file in lower; copy-up must restore the copy's owner to root and then write `trusted.overlay.origin` — executed with the caller's credentials, these two steps would inevitably fail.

**The scenario this design targets**. Container image layers are built by root, while the running container executes as an ordinary user: an ordinary user's first write to a lower object, the faithful attribute copying of copy-up, and the attribute carrying of a clear-empty exchange all happen inside the ordinary user's session. If internal IO cannot run with the mounter's credentials, faithful copying either interrupts the whole operation with `EACCES` or silently drops the `security.*`/`trusted.*` attributes — the former breaks usability, the latter breaks the commitment to copy fidelity.

**Where the change points would land if implemented**. `inode/xattr.rs`: the path that copies xattrs at copy-up needs an explicit credential source (in the current implementation this path performs its reads and writes with the caller's credentials); `inode/copyup/mod.rs`: the metadata/xattr/timestamp transfer of the promote preparation stage should run as a whole within the scope of the mounter's credentials; `inode/dir/remove.rs`: the xattr copying of the clear-empty exchange should likewise run within the scope of the mounter's credentials; on the mount side, the construction flow (`fs/mount/mod.rs`) captures a snapshot of the mounter's credentials; on the VFS side, a call path for overlay-internal IO is needed, bypassing precisely the caller-credential dependencies at `fs/vfs/path/mod.rs` (the permission checks for directory-entry changes and xattr operations are embedded there and use the caller's credentials) and `fs/vfs/path/dentry.rs` (the sticky check).

**The VFS/Kernel-level gap**. A backend-call variant without caller context is needed — that is, an internal-call variant of DirDentry — together with a permission-check mechanism that distinguishes caller requests from overlay-internal IO: let permission checking and backend execution be split into two phases, in which the public Path/DirDentry methods keep completing the permission check as the caller and serving caller requests, while overlay-internal IO takes the internal-call variant that skips the caller permission check; also needed are a data structure that can hold the mounter's credential snapshot and a mechanism for entering and restoring that credential scope (this design's two-phase permission check has already decoupled Inode-level backend calls from task credentials; the gap is concentrated at the Path/DirDentry layer and in the copy-up preparation stage); as for the LSM hook that obtains a transitional label for the new copy before copy-up, this platform has no corresponding interface yet.

#### 2. redirect_dir (renaming a directory across parents)

**Mechanism sketch**. When a directory coming from lower is renamed to a different parent directory (a cross-parent rename), the lower layer is not writable; if the directory were merely moved within upper, its merge relationship with the same-named directory in lower would be severed. With redirect_dir enabled, overlay instead completes the rename in the following steps:

1. copy-up the directory itself to obtain an upper copy;
2. write the `trusted.overlay.redirect` marker on the upper copy, its value being the directory's original overlay path before the rename;
3. rename the copy to the new name;
4. leave a whiteout at the old name.

From then on, the directory at the new name keeps merging with the lower directory at the path the redirect record points to. Example: lower has the directory `/a/d`, and the user renames `d` to `/b/d` — upper's `/b/d` carries the redirect record `/a/d`, and reading the contents of `/b/d` follows the record back to `/a/d` to merge the lower contributions.

**The scenario this design targets**. The integrity of directory renaming: without redirect_dir, moving a lower/merged directory across parents always returns `EXDEV`, so the basic operation "move a directory" is unavailable for half of the directories (all those with a lower source); containers and build systems reorganize directories heavily, and `EXDEV` forces them back to copying whole directories.

**Where the change points would land if implemented**. `inode/dir/rename.rs`: the TODO(redirect_dir) hook (`:60-64`) in `cross_device_gate` is the trigger point — when the source is a lower/merged directory, the rename crosses parents, redirect_dir=on, and upper supports xattr, the default behavior of returning `EXDEV` directly is replaced with completing the rename via the redirect_dir mechanism; the lock-ordering pre-study has already reached a conclusion: when rewriting the publication coordinates of an object that has not yet completed copy-up, one must first take that object's copy-up mutex and then the parent directory's transaction lock. `inode/xattr.rs`: the marker name of the redirect record is already in the known-suffix table (`:51-54`). `inode/lookup.rs`: the lookup/merge side is purely overlay-internal implementation — when projecting a directory, read the redirect record, resolve the lower directory from the recorded path, and bring it into the merge. The creation side needs the full overlay path of the source directory to compute the redirect value; in this design the authoritative basis for recording directory coordinates is the `recorded_parent` chain.

**The VFS/Kernel-level gap**. The authoritative source of the "full overlay path" on the creation side depends on the interface that substitutes a dentry for an inode (the alternative described in #5): this interface is planned on this platform as an independent PR, and until it merges, overlayfs does not derive the full overlay path from the dentry, with `recorded_parent` remaining the canonical field for recording parent-directory coordinates. Linux assembles this path from the `d_parent` chain; this platform has no equivalent authoritative interface for reading parent directories level by level. How the length limit of the redirect value itself (a rule akin to Linux's `redirect_max`) interacts with the escaping mechanism also needs an explicit validation rule.

#### 3. index (hard-link sharing and identity correspondence)

**Mechanism sketch**. The filesystem hosting upper maintains an index directory; each index entry is a real inode keyed by the lower object's handle (file handle). When a non-directory whose hard-link count is greater than 1 (nlink>1) undergoes its first copy-up, overlay publishes the copy directly as an index entry in index and then creates, at the real position in upper, a hard link pointing to it; from then on, the remaining hard-link names of the same lower object query index with their respective lower handles at lookup, and on a hit the existing upper inode is adopted as their upper alias — zero-copy sharing. After all aliases have been deleted, the corresponding index entry in index turns into a whiteout. The basis of crash safety is that an index entry is itself a real inode: at any moment an index entry either exists completely or does not exist at all; no half-finished form exists. `st_nlink` is corrected by way of the `trusted.overlay.nlink` marker according to the formula: physical nlink − upper baseline + lower baseline.

**The scenario this design targets**. The many hard-linked files in image layers (artifacts of build tools): without index, each name copy-ups on its own, space inflates multiplicatively, and the hard-link names of the same lower object become independent of one another from then on — modify one of them, and the other aliases do not see it; `st_nlink` is incorrect as well. index converges the hard-link names of the same lower object onto the same upper inode, keeping nlink semantics and space usage faithful at the same time.

**Where the change points would land if implemented**. `inode/identity.rs`: the data structure carrying the key already exists — `LowerIdOrigin` (a 32-byte serialized record of `container_dev_id` + `lower_layer_root_ino` + `real_ino`, `:367-392`) takes the place of Linux's exportfs file handle; the TODO(origin-verify) (`:337-338`) on `origin_real_ino_resolves` marks the upgrade direction of the handle-resolution capability (resolving an inode back out of a record). `inode/copyup/mod.rs`: the branch that publishes nlink>1 non-directories into index at copy-up. `inode/lookup.rs` / `inode/inode_cache.rs`: the lookup side queries index and, on a hit, adopts the existing upper inode as the alias. `inode/dir/link.rs` / `inode/dir/remove.rs`: the publication of aliases (hard-link names), and the turning of the index entry into a whiteout once the last alias has been deleted. `inode/xattr.rs`: the marker name used for the nlink correction is already in the known-suffix table (`:51-54`).

**The VFS/Kernel-level gap**. index depends on an infrastructure of "handles that identify lower objects", and which alternative to use requires a decision first: Linux uses the exportfs file handle (an inode can be resolved back out of the handle), whereas this design's origin record is a 32-byte triple that supports only identity comparison and not resolution — enough for index hits, but fully aligning with Linux's export semantics requires a separate decision. The nlink correction above requires synchronizing the physical count and the semantic count across the several operations link/unlink/copy-up (this design currently explicitly does not maintain merged nlink, see the module comment of `inode/dir/rename.rs`); this count-synchronization protocol itself needs an independent design.

#### 4. metacopy (copy-up with metadata first and data backfilled later)

**Mechanism sketch**. With metacopy enabled, copy-up moves only metadata: upper publishes a file entry without data, stamped with the `trusted.overlay.metacopy` marker, while the data stays in lower; the first time the object is opened in a writable fashion, overlay backfills the data into the already-existing upper inode and removes the marker. A single copy-up is thereby split into two stages: metadata comes from upper, and the data still comes from lower. The benefiting scenario is the "metadata-only change" workloads in container image layers (chown/chmod/timestamp adjustments): the cost of copy-up drops from O(data) to O(metadata). There are three costs: reading data requires resolving the data in lower across layers; the trust premise widens — an untrusted layer can forge redirect/metacopy markers and point data access at an arbitrary lower object (the Linux documentation explicitly warns against enabling this with untrusted layers); and metacopy depends on redirect_dir=on and conflicts with nfs_export, so Linux disables metacopy by default and makes it an explicit mount option (the dependency check in `fs/overlayfs/params.c:913-922`; the metacopy chapter of `overlayfs.rst`).

**The scenario this design targets**. Image-layering deduplication and startup latency: many layers in an image only copy files or adjust permissions and never rewrite data; metacopy makes both the mount cost and the first-write cost of such layers independent of file size.

**Where the change points would land if implemented**. `inode/copyup/mod.rs`: promote gains a branch that publishes only metadata (skipping the data stream and writing the metacopy marker), together with the convergence step of "backfill the data + remove the marker" the first time the object is opened in a writable fashion; `inode/data.rs`: the read path needs to tell the source of metadata from the source of data — when the upper side has no data, resolve and read the data in lower; `inode/xattr.rs`: the metacopy marker name is already in the known-suffix table (`:51-54`).

**The VFS/Kernel-level gap**. metacopy depends on revising this design's "single publication" contract: this design commits to copy-up being "a one-way, at-most-once lower→upper publication" (the `upper: Once<RealObject>` of #5), whereas metacopy splits a single publication into the two stages of "metadata first, data backfilled later", so the publication semantics of copy-up need a revised design; the revision of the trust premise (accepting that "markers may be forged and point data access at an arbitrary lower object") and the hard dependency on redirect_dir (implement item 2 of this chapter first) stand together as prerequisites.
