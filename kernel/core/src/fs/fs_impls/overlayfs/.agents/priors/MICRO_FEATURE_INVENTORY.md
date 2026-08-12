<!-- SPDX-License-Identifier: MPL-2.0 -->

# Micro-Feature Inventory — Overlayfs

This is a flat, priority-ordered list of overlayfs micro-features. Each entry is
precise enough that a Creator Pass can name it as part of an explicit covered
set, and a Checker Pass can validate it independently.

**Priority tiers** (a tier must be fully covered before the next tier starts;
within a tier, order is a suggested implementation sequence, not a hard
dependency):

- **P0 — Mandatory core**: without these, overlayfs is not a filesystem.
  Completing P0 yields a read-only overlay that can mount and stat. P0 is
  **functionally** complete but **not security-complete**: it does not enforce
  the two-step permission model (Spec §9), so a P0-only overlay could allow
  reads the current task should not be permitted. P1-18 (permission check)
  must follow immediately to close this gap.
- **P1 — Basic usability**: without these, overlayfs cannot be written to or
  used as a real working directory. Completing P0+P1 yields a "basically
  usable" overlayfs (the milestone the user asked for): mounts, reads, writes,
  creates, deletes, renames (with EXDEV fallback), and the two-step permission
  check. Only optional extensions remain.
- **P2 — POSIX completeness**: features that bring overlayfs closer to
  standards-compliant behavior. Optional for basic usability; needed for
  stricter workloads.
- **P3 — Advanced extensions**: features that significantly complicate the
  implementation (NFS export, index, metacopy, verity, volatile, FD layers).
  Explicitly out of scope for the initial refactor waves.

Meso-component grouping is NOT done here — that is the Architect's job when
building the Bi-Directional Traceability Matrix. Each micro-feature is tagged
with the spec section (`Spec §`) and reference implementation file(s) it maps
to, so the Architect can cluster them later.

Legend for tags:
- `Spec §`: section in `FILESYSTEM_SPEC_SUMMARY.md`
- `Ref`: file(s) in `~/linux/fs/overlayfs/` (see `REFERENCE_IMPLEMENTATION_SUMMARY.md` §2)
- `Lock`: lock domain(s) involved — `VFS` (overlay parent dir consistency),
  `INODE` (`ovl_inode->lock`), `WL` (`whiteout_lock`), `IU` (inuse:
  upper/workdir exclusivity), `CUL` (copy-up coordination, distinct from
  `IU`), `UPPER` (upper-layer dir consistency), `NONE` (lockless / atomic-only)

**Asterinas lock constraints (HARD RULES for Architect/Designer/Creator):**

1. **Only `ostd::sync::Mutex` is a safe sleep lock.** `ostd::sync::RwLock` is
   spin-based (`PreemptDisabled` guard; documented as "Spin-based Read-write
   Lock" in `ostd/src/sync/rwlock.rs`). `ostd::sync::SpinLock` is obviously
   spin. Therefore ANY critical section that may trigger BIO (block I/O,
   `Bio` calls, `read_blocks`/`write_blocks`, `VmIo`, `BioWaiter::wait`,
   `page_cache` eviction, or any call into the upper/lower fs that may
   sleep) MUST use `Mutex`, never `RwLock` or `SpinLock`.
2. **Asterinas VFS does NOT hold a parent-directory lock when invoking inode
   ops** (`lookup`/`create`/`unlink`/`rename`/...). This is a fundamental
   divergence from Linux, where VFS holds `i_rwsem` on the parent. The
   `VFS` lock domain in this inventory therefore does NOT mean "VFS-held
   lock we can rely on"; it means "overlay parent dir consistency, which
   the overlay MUST establish itself." The Architect must decide how the
   overlay serializes concurrent directory mutations (e.g., a per-overlay-dir
   `Mutex` taken at the entry of each mutating inode op). This is a new
   lock domain not present in Linux overlayfs and must be added to the
   Global Lock Topology.
3. **No reentrant locks exist in Asterinas.** Any code path that may
   re-enter the same lock (e.g., a VFS callback that calls back into the
   same fs) must be structured to release the lock before the reentrant
   call, or use a different lock granularity.
4. **`INODE`, `CUL`, `UPPER` all cross BIO** → must be `Mutex`.
5. **`WL` short critical section, no BIO** → may use `SpinLock` or `Mutex`;
   `Mutex` is safer and matches Linux.
6. **`IU` is a mount-time bit** → `AtomicBool` + waitqueue, not a kernel
   mutex.

---

## P0 — Mandatory Core (read-only overlay that mounts and stats)

### P0-01 Mount option parsing (minimal)
- **What**: parse `lowerdir`, `upperdir`, `workdir` (upper/work optional for
  read-only mount). Reject unknown options. Store in `ovl_config`.
- **Spec §**: 2, 10
- **Ref**: `params.c`, `params.h`
- **Lock**: NONE

### P0-02 Layer stack assembly
- **What**: resolve `lowerdir`/`upperdir` paths to dentries, build
  `ovl_layer[]` (idx 0 = upper), assign fsid per unique underlying sb, set up
  minimal per-layer traps (full trap lifecycle is P3-08). Verify upper fs
  supports xattr + d_type (reject NFS as upper).
- **Spec §**: 2
- **Ref**: `super.c` (`ovl_get_upper`, `ovl_get_layers`, `ovl_get_lowerstack`)
- **Lock**: VFS, IU

### P0-03 Workdir setup
- **What**: when `upperdir` is given, create/verify `workdir` on the same fs
  as upperdir; take `workdir_locked`. Skip when read-only (no upper).
- **Spec §**: 2, 10
- **Ref**: `super.c` (`ovl_make_workdir`, `ovl_get_workdir`)
- **Lock**: VFS, IU

### P0-04 Root dentry/inode construction
- **What**: build root `ovl_entry` (lower stack from root dirs), allocate root
  `ovl_inode` via `ovl_get_inode`, set `d_op`. Root is a merged dir if upper
  root + lower root both exist.
- **Spec §**: 1, 4
- **Ref**: `super.c` (`ovl_get_root`, `ovl_get_lowerstack`), `inode.c`
  (`ovl_get_inode`, `ovl_new_inode`)
- **Lock**: VFS

### P0-05 Superblock operations
- **What**: implement `put_super`, `sync_fs`, `statfs`, `show_options`. Forward
  `sync_fs`/`statfs` to upper fs (or topmost lower for read-only).
- **Spec §**: 1
- **Ref**: `super.c` (`ovl_put_super`, `ovl_sync_fs`, `ovl_statfs`)
- **Lock**: NONE

### P0-06 Overlay inode facade (data structures)
- **What**: define `ovl_inode` with `__upperdentry` (READ_ONCE), `oe`,
  `flags`, `version`, `lock` (mutex). Define `ovl_entry` with `__lowerstack`
  flex array. Define `ovl_path` `{ layer, dentry }`. Define `ovl_layer`,
  `ovl_sb`.
- **Spec §**: 1
- **Ref**: `ovl_entry.h`, `overlayfs.h`
- **Lock**: NONE (definition only)

### P0-07 Path-type computation
- **What**: `ovl_path_type(dentry)` returns upper/merge/origin flags.
  `ovl_path_upper`/`lower`/`lowerdata`/`real`/`realdata` accessors.
  `ovl_dentry_upper`/`lower`/`lowerdata`/`real`. `ovl_inode_upper`/`lower`/
  `lowerdata`/`real`/`realdata`.
- **Spec §**: 1, 2
- **Ref**: `util.c`
- **Lock**: NONE (atomic reads)

### P0-08 Layer-ordered lookup (non-directory)
- **What**: `ovl_lookup` iterates layers top→bottom. For a non-directory name,
  first hit wins; lower hits hidden. Build `ovl_entry` lower stack. Allocate
  overlay inode. Set dentry flags.
- **Spec §**: 2, 4
- **Ref**: `namei.c` (`ovl_lookup`, `ovl_lookup_single`, `ovl_lookup_layer`)
- **Lock**: VFS (parent i_rwsem held by VFS)

### P0-09 Layer-ordered lookup (directory merge)
- **What**: when a name is a directory in multiple layers and not opaque,
  store all hits in `ovl_entry.__lowerstack` (merge). Upper dir wins for
  metadata. Set `OVL_E_UPPER_ALIAS` if upper exists.
- **Spec §**: 4
- **Ref**: `namei.c` (`ovl_lookup`)
- **Lock**: VFS

### P0-10 Opaque directory handling in lookup
- **What**: if an upper dir has `overlay.opaque=y`, stop merging — do not
  search lower layers for that name. Lower dirs are hidden.
- **Spec §**: 5.2
- **Ref**: `namei.c` (`ovl_lookup_single` opaque check), `xattrs.c`
- **Lock**: VFS, UPPER (xattr read)

### P0-11 Whiteout handling in lookup
- **What**: if a whiteout (char dev 0/0 OR xattr whiteout) is found in upper
  at a name, the name is hidden from readdir and lookup returns `ENOENT`.
  Detect xwhiteout dirs via `overlay.opaque=x` marker.
- **Spec §**: 5.1
- **Ref**: `namei.c`, `util.c` (`ovl_is_whiteout`, `ovl_path_is_whiteout`)
- **Lock**: VFS, UPPER

### P0-12 getattr / stat with dev/ino mapping
- **What**: `ovl_getattr` reports `st_dev`/`st_ino` per the configured mode
  (same-fs uniform, or overlay-internal dev + per-layer ino, or xino — see
  P2-01). `st_nlink` from real inode (with nlink xattr override if present).
  `st_mode`/`uid`/`gid`/`size`/`blocks`/timestamps from real upper-or-lower
  inode.
- **Spec §**: 3
- **Ref**: `inode.c` (`ovl_getattr`, `ovl_map_dev_ino`)
- **Lock**: NONE (atomic read of real inode attrs)

### P0-13 Readdir on non-merged directory
- **What**: if the overlay dir is not merged (pure upper or pure lower),
  delegate readdir directly to the underlying real dir.
- **Spec §**: 4
- **Ref**: `readdir.c` (`ovl_dir_is_real` fast path)
- **Lock**: VFS (dir i_rwsem), UPPER

### P0-14 Readdir on merged directory (cache + dedup)
- **What**: build `ovl_dir_cache` (rb-tree + list). Read upper first, then
  each lower layer top→bottom, dedup by name. Skip whiteouts. Cache in
  `ovl_inode->cache` under `ovl_inode->lock`, version-checked against
  `ovl_inode->version`. `seekdir(0)` discards cache.
- **Spec §**: 4
- **Ref**: `readdir.c` (`ovl_cache_get`, `ovl_dir_read_merged`, `ovl_fill_merge`)
- **Lock**: VFS, INODE (cache swap), UPPER (read upper dir)

### P0-15 d_ino computation in readdir
- **What**: compute `d_ino` for each cached entry. Without xino: upper entries
  use overlay ino; lower entries use lower real ino (or overlay ino if same
  fs). With xino (P2-01): remap via `ovl_remap_lower_ino`.
- **Spec §**: 3, 4
- **Ref**: `readdir.c` (`ovl_calc_d_ino`, `ovl_remap_lower_ino`)
- **Lock**: NONE (computed from cached entries)

### P0-16 Inode cache + inode construction
- **What**: dedicated inode slab cache. `ovl_alloc_inode`/`free_inode`.
  `ovl_get_inode` constructs from `ovl_inode_params` (upperdentry, oe, redirect,
  lowerdata_redirect). `ovl_new_inode` for fresh inodes. `ovl_lookup_inode`
  finds existing overlay inode by real dentry.
- **Spec §**: 1
- **Ref**: `super.c` (`ovl_alloc_inode`, `ovl_inode_cachep`), `inode.c`
  (`ovl_get_inode`, `ovl_new_inode`, `ovl_lookup_inode`)
- **Lock**: NONE (slab), VFS (inode hash)

### P0-17 Dentry operations + revalidate
- **What**: `d_op` set on overlay dentries. `d_revalidate`/`weak_revalidate`
  detect layer mountpoint changes (underlying dentry stale) and force
  re-lookup. Casefold variant if layer fs supports encoding.
- **Spec §**: 2
- **Ref**: `super.c` (`ovl_dentry_operations`, `ovl_dentry_revalidate`)
- **Lock**: NONE

### P0-18 Read-only mount enforcement
- **What**: if no `upperdir`, the overlay is read-only. Mutative inode ops
  (`create`/`mkdir`/`unlink`/`rename`/`setattr`/`write_at`/...) return `EROFS`.
- **Spec §**: 2
- **Ref**: `super.c` (`ovl_force_readonly`), all mutative ops
- **Lock**: NONE (check at op entry)

---

## P1 — Basic Usability (writable overlay, basic file ops)

### P1-01 Copy-up coordination (locking)
- **What**: per-dentry bit lock (`ovl_copy_up_start`/`end`) ensures only one
  task copies up a given dentry; others wait and see
  `ovl_already_copied_up`. `OVL_UPPERDATA` flag marks "has upper data".
- **Spec §**: 6
- **Ref**: `util.c` (`ovl_copy_up_start`, `ovl_copy_up_end`,
  `ovl_already_copied_up`)
- **Lock**: CUL (d_fsdata bit, distinct from IU)

### P1-02 Copy-up trigger detection
- **What**: `ovl_maybe_copy_up(dentry, flags)` checks write-intent flags
  (`O_WRONLY`/`O_RDWR`/`O_TRUNC`/`O_APPEND`) and metadata-change conditions.
  Fast path: `ovl_already_copied_up` skips if upper exists. Note: creating a
  symlink does NOT trigger copy-up (symlink target is metadata, but symlink
  creation is a name op, not a write to an existing object).
- **Spec §**: 6
- **Ref**: `copy_up.c` (`ovl_maybe_copy_up`, `ovl_copy_up`,
  `ovl_copy_up_with_data`), `overlayfs.h` (`ovl_open_flags_need_copy_up`)
- **Lock**: INODE, CUL

### P1-03 Copy-up parent recursion
- **What**: before copying up an object, ensure its parent directory (and
  ancestors) are copied up. Recursive `ovl_copy_up` on parent path.
- **Spec §**: 6
- **Ref**: `copy_up.c` (parent recursion in `ovl_copy_up`)
- **Lock**: INODE, CUL

### P1-04 Full copy-up via workdir temp + atomic rename
- **What**: `ovl_copy_up_workdir`: create temp inode in workdir, copy
  metadata + data + xattrs, `fsync` (per `fsync_mode`), atomically rename into
  upper dir. Set `overlay.origin` xattr. Update `__upperdentry` (WRITE_ONCE).
- **Spec §**: 6, 13
- **Ref**: `copy_up.c` (`ovl_copy_up_workdir`, `ovl_copy_up_data`,
  `ovl_copy_up_metadata`, `ovl_link_up`)
- **Lock**: INODE, CUL, UPPER (workdir + target dir i_rwsem)

### P1-05 Copy-up metadata (owner/mode/timestamps)
- **What**: `ovl_set_attr` applies owner/mode/timestamps from lower stat to
  upper dentry. `ovl_set_timestamps`, `ovl_set_size`.
- **Spec §**: 6
- **Ref**: `copy_up.c` (`ovl_set_attr`, `ovl_set_timestamps`, `ovl_set_size`)
- **Lock**: UPPER

### P1-06 Copy-up xattrs
- **What**: `ovl_copy_xattr` copies user xattrs from lower to upper, filtered
  by `ovl_must_copy_xattr` (skip overlay-private xattrs). ACL copy via
  `ovl_copy_acl`.
- **Spec §**: 6, 20
- **Ref**: `copy_up.c` (`ovl_copy_xattr`, `ovl_copy_acl`,
  `ovl_must_copy_xattr`)
- **Lock**: UPPER

### P1-07 Origin FH encode + store
- **What**: `ovl_encode_real_fh` encodes lower inode to `ovl_fh`.
  `ovl_set_origin_fh` stores it in `overlay.origin` xattr on the new upper
  dentry. Used for index (P3) and offline-change detection (P2).
- **Spec §**: 6, 16
- **Ref**: `copy_up.c` (`ovl_encode_real_fh`, `ovl_get_origin_fh`,
  `ovl_set_origin_fh`)
- **Lock**: UPPER

### P1-08 File open (real file selection)
- **What**: `ovl_open` decides real file: upper if copied up, else lower.
  Triggers copy-up if write-intent flags. Allocates `struct ovl_file` wrapper
  holding the real file. `O_NOATIME` on lower opens (no atime update per
  Spec §19a). Known divergence (Spec §19c): opening an executing lower file
  for write/truncate is NOT denied with `ETXTBSY` — documented limitation.
- **Spec §**: 6, 19
- **Ref**: `file.c` (`ovl_open`, `ovl_open_realfile`, `ovl_file_alloc`,
  `ovl_real_file`)
- **Lock**: INODE (copy-up), UPPER (open real file)

### P1-09 File release
- **What**: `ovl_release` drops real file refs, frees `ovl_file`.
- **Spec §**: 6
- **Ref**: `file.c` (`ovl_release`, `ovl_file_free`)
- **Lock**: NONE

### P1-10 File read/write delegation
- **What**: `ovl_read_iter`/`ovl_write_iter` delegate to real file with
  `creator_cred` override. `O_APPEND` handled by real file. Write triggers
  copy-up (P1-08) before delegation.
- **Spec §**: 6, 9
- **Ref**: `file.c` (`ovl_read_iter`, `ovl_write_iter`)
- **Lock**: UPPER (real file I/O)

### P1-11 File llseek delegation
- **What**: `ovl_llseek` delegates to real file. Handle copy-up awareness
  (seek on lower file is fine; no special case unless mmap'd).
- **Spec §**: 6
- **Ref**: `file.c` (`ovl_llseek`)
- **Lock**: UPPER

### P1-12 File mmap delegation
- **What**: `ovl_mmap` delegates to real file. `MAP_SHARED` writable requires
  upper file (copy-up first). `MAP_SHARED` of lower file is read-only and
  does NOT see later changes (Spec §19b).
- **Spec §**: 6, 19
- **Ref**: `file.c` (`ovl_mmap`)
- **Lock**: INODE (copy-up), UPPER

### P1-13 File fsync delegation
- **What**: `ovl_fsync` delegates to real file. Honor `fsync_mode` (default
  `auto`: just delegate; `strict`: also sync parent dir; `volatile`: no-op).
- **Spec §**: 13
- **Ref**: `file.c` (`ovl_fsync`)
- **Lock**: UPPER

### P1-14 File fallocate / fadvise delegation
- **What**: `ovl_fallocate`/`ovl_fadvise` delegate to real file. Fallocate
  triggers copy-up (modifies file).
- **Spec §**: 6
- **Ref**: `file.c` (`ovl_fallocate`, `ovl_fadvise`)
- **Lock**: INODE (copy-up), UPPER

### P1-15 Splice read/write delegation
- **What**: `ovl_splice_read`/`ovl_splice_write` delegate to real file. Write
  variant triggers copy-up.
- **Spec §**: 6
- **Ref**: `file.c` (`ovl_splice_read`, `ovl_splice_write`)
- **Lock**: INODE (copy-up), UPPER

### P1-16 setattr (chmod/chown/utimes)
- **What**: `ovl_setattr` triggers copy-up, then forwards `notify_change` to
  upper dentry (via `ovl_do_notify_change` with idmapping).
- **Spec §**: 6, 9
- **Ref**: `inode.c` (`ovl_setattr`)
- **Lock**: INODE (copy-up), UPPER

### P1-17 update_time (atime/mtime/ctime)
- **What**: `ovl_update_time` triggers copy-up (for lower files), then
  forwards timestamp update to upper. Note: atime on lower files is NOT
  updated (Spec §19a).
- **Spec §**: 6, 19
- **Ref**: `inode.c` (`ovl_update_time`)
- **Lock**: INODE, UPPER

### P1-18 Permission check (two-step)
- **What**: `ovl_permission` performs (a) local DAC+MAC on overlay inode with
  current creds, then (b) real check on underlying inode with stashed
  `creator_cred`. `default_permissions` skips (b).
- **Spec §**: 9
- **Ref**: `inode.c` (`ovl_permission`), `util.c` (`ovl_override_creds`,
  `ovl_creds`, `with_ovl_creds`)
- **Lock**: NONE (checks only)

### P1-19 Credential stashing + override
- **What**: at mount, stash `creator_cred` in `ovl_fs`. `ovl_override_creds`
  switches to stashed creds for underlying VFS calls. `with_ovl_creds(sb)`
  scoped guard.
- **Spec §**: 9
- **Ref**: `util.c` (`ovl_override_creds`, `ovl_creds`), `super.c`
  (`ovl_fill_super_creds`)
- **Lock**: NONE (creds are per-task)

### P1-20 Write access accounting
- **What**: `ovl_get_write_access`/`put_write_access`, `ovl_want_write`/
  `drop_write`, `ovl_start_write`/`end_write` manage write refcount on the
  upper sb (freeze protection).
- **Spec §**: 6
- **Ref**: `util.c`
- **Lock**: NONE (atomic counters on upper sb)

### P1-21 Directory create (upper-only path)
- **What**: `ovl_create_upper` creates directly in upper dir when no
  conflicting lower entry. `ovl_instantiate` links new upper dentry to overlay
  inode, updates `__upperdentry`.
- **Spec §**: 6
- **Ref**: `dir.c` (`ovl_create_upper`, `ovl_instantiate`)
- **Lock**: VFS (parent), UPPER (create)

### P1-22 Directory create over whiteout
- **What**: `ovl_create_over_whiteout`: when a whiteout exists in upper at
  the target name, create temp in workdir, atomically rename over the
  whiteout. Clean up the whiteout (takes `whiteout_lock`).
- **Spec §**: 5.1, 6
- **Ref**: `dir.c` (`ovl_create_over_whiteout`, `ovl_cleanup_and_whiteout`)
- **Lock**: VFS, INODE, WL, UPPER

### P1-23 Directory create object dispatcher
- **What**: `ovl_create_object` / `ovl_create_or_link` decide upper-only vs
  over-whiteout based on whether a lower entry exists. Dispatch to
  `ovl_create_upper` or `ovl_create_over_whiteout`. Handle hardlink case.
  When creating a directory over an existing lower directory (not a whiteout),
  set `overlay.opaque=y` on the new upper dir to prevent merge with the lower
  dir (Spec §5.2).
- **Spec §**: 5.2, 6
- **Ref**: `dir.c` (`ovl_create_object`, `ovl_create_or_link`,
  `ovl_create_handle_whiteouts`)
- **Lock**: VFS, INODE, WL, UPPER

### P1-24 create / mkdir / mknod / symlink inode ops
- **What**: `ovl_create`/`ovl_mkdir`/`ovl_mknod`/`ovl_symlink` — the
  `inode_operations` entries. Thin wrappers around `ovl_create_object` with
  the right `ovl_cattr`.
- **Spec §**: 6
- **Ref**: `dir.c`
- **Lock**: VFS, UPPER

### P1-25 Whiteout creation
- **What**: `ovl_whiteout` creates a char-dev-0/0 whiteout in upper (or
  xattr whiteout if upper fs doesn't support char devs). Used by unlink/rmdir.
- **Spec §**: 5.1
- **Ref**: `dir.c` (`ovl_whiteout`, `ovl_cleanup_and_whiteout`)
- **Lock**: VFS, WL, UPPER

### P1-26 unlink
- **What**: `ovl_unlink`: if object has upper, `ovl_do_unlink` it; create
  whiteout in upper to hide lower entry. If pure-upper (no lower), just
  unlink (no whiteout needed).
- **Spec §**: 5.1, 6
- **Ref**: `dir.c` (`ovl_unlink`)
- **Lock**: VFS, WL, UPPER

### P1-27 rmdir
- **What**: `ovl_rmdir`: verify dir is empty (`ovl_check_empty_dir` respecting
  whiteouts). If upper exists, `ovl_do_rmdir`; create whiteout. If pure-upper,
  just rmdir.
- **Spec §**: 5.1, 6
- **Ref**: `dir.c` (`ovl_rmdir`), `readdir.c` (`ovl_check_empty_dir`)
- **Lock**: VFS, WL, UPPER

### P1-28 link (hardlink)
- **What**: `ovl_link`: copy up the target if needed, then `ovl_do_link` in
  upper. Without `index` (P3), this "breaks" the link on copy-up of a
  multi-linked lower file (Spec §19 / index discussion).
- **Spec §**: 6
- **Ref**: `dir.c` (`ovl_link`)
- **Lock**: VFS, INODE (copy-up), CUL, UPPER

### P1-29 rename (same-directory, non-directory)
- **What**: `ovl_rename` for files within the same directory: copy up if
  needed, `ovl_do_rename` in upper. Handle whiteout at target.
- **Spec §**: 6
- **Ref**: `dir.c` (`ovl_rename`)
- **Lock**: VFS (both parents), INODE, UPPER

### P1-30 rename (cross-directory, EXDEV default)
- **What**: for directories that are lower/merged, return `EXDEV` unless
  `redirect_dir` is enabled (P2-04). For non-dirs and pure-upper dirs, allow
  cross-dir rename via `ovl_do_rename`.
- **Spec §**: 7
- **Ref**: `dir.c` (`ovl_rename` EXDEV check)
- **Lock**: VFS, UPPER

### P1-31 Directory modification invalidates readdir cache
- **What**: after create/unlink/rename/mkdir/rmdir on a directory, bump
  `ovl_inode->version` and free `ovl_dir_cache` so next readdir rebuilds.
  `ovl_dir_modified(dentry, impurity)`.
- **Spec §**: 4
- **Ref**: `util.c` (`ovl_dir_modified`, `ovl_set_dir_cache`,
  `ovl_dir_cache_free`)
- **Lock**: INODE

### P1-32 Symlink read (get_link)
- **What**: `ovl_get_link` reads symlink target. For lower symlink, copy-up
  first (target is metadata), then read from upper.
- **Spec §**: 6
- **Ref**: `inode.c` (`ovl_get_link`)
- **Lock**: INODE (copy-up), UPPER

### P1-33 xattr get/set/list delegation
- **What**: `ovl_xattr_handlers` route get/set/list to the real upper dentry
  (after copy-up for lower objects). Overlay-private xattrs
  (`overlay.*`) are filtered from `listxattr`. `ovl_is_private_xattr`
  classification.
- **Spec §**: 20
- **Ref**: `xattrs.c` (`ovl_own_xattr_get`/`set`, `ovl_other_xattr_get`/`set`,
  `ovl_listxattr`, `ovl_is_private_xattr`)
- **Lock**: INODE (copy-up), UPPER

### P1-34 Workdir temp helpers
- **What**: `ovl_create_real`/`ovl_create_temp`/`ovl_cleanup`/`ovl_tempname`
  for creating/cleaning temp inodes in workdir during copy-up and
  create-over-whiteout.
- **Spec §**: 6
- **Ref**: `dir.c` (`ovl_create_real`, `ovl_create_temp`, `ovl_cleanup`,
  `ovl_tempname`)
- **Lock**: UPPER (workdir)

### P1-35 Inuse lock (upper/workdir exclusivity)
- **What**: `ovl_inuse_trylock`/`unlock`/`is_inuse` prevent two overlays from
  using the same upper/workdir. Taken at mount; checked during copy-up
  coordination.
- **Spec §**: 16
- **Ref**: `util.c` (`ovl_inuse_trylock`, `ovl_inuse_unlock`, `ovl_is_inuse`)
- **Lock**: IU

### P1-36 Shared whiteout cache
- **What**: `ovl_fs->whiteout` + `whiteout_lock` cache a reusable whiteout
  dentry to speed up whiteout creation. `ovl_do_whiteout` uses it.
- **Spec §**: 5.1
- **Ref**: `dir.c` (`ovl_whiteout`), `util.c`, `ovl_entry.h`
- **Lock**: WL

### P1-37 page_cache forwarding + copy-up trigger
- **What**: `OverlayInode::page_cache()` returns the **upper inode's**
  `PageCache` (after copy-up). If the object is on a lower layer, copy-up
  is triggered first (mmap of a lower file needs a writable upper page).
  The overlay inode does NOT implement `PageCacheBackend` itself; it is a
  pure forwarder. This avoids a double page-cache layer and keeps cache
  coherence the upper fs's responsibility.
- **Spec §**: 6, 19
- **Ref**: `file.c` (Linux `ovl_file_operations` does not register a page
  cache; VFS default walks through to the real upper file), legacy
  `overlayfs/fs.rs:492-497` (`page_cache()` forwards to upper after
  `build_upper_recursively_if_needed`)
- **Lock**: INODE (copy-up), CUL, UPPER (upper `page_cache()` may touch upper
  inode state)

---

## P2 — POSIX Completeness (optional for basic usability)

### P2-01 xino (unified st_ino/st_dev)
- **What**: encode fsid into high bits of `st_ino`. `xino=off`/`auto`/`on`.
  Auto enables only if all lowers support NFS FH. Handle overflow fallback.
- **Spec §**: 3
- **Ref**: `readdir.c` (`ovl_remap_lower_ino`), `inode.c` (`ovl_map_dev_ino`),
  `super.c` (`xino_mode`)
- **Lock**: NONE (computation)

### P2-02 redirect_dir (directory rename without EXDEV)
- **What**: `redirect_dir=on`/`follow`/`nofollow`/`off`. On cross-dir dir
  rename: copy up dir (metadata only), set `overlay.redirect` xattr to
  original path, move dir. Lookup follows redirect xattr. Enforce
  `ovl_redirect_max` module param (default 256) cap on redirect path length
  (was P3-10, folded here).
- **Spec §**: 7
- **Ref**: `dir.c` (`ovl_set_redirect`, `ovl_redirect_max`), `namei.c`
  (`ovl_check_redirect`, `ovl_get_redirect_xattr`), `params.c`
- **Lock**: VFS, INODE, UPPER

### P2-03 OVL_IMPURE flag + impure xattr
- **What**: `OVL_IMPURE` marks an upper dir that may contain non-pure-upper
  entries (i.e., has lower aliases). `ovl_set_impure` sets `overlay.impure`
  xattr. Used to optimize readdir cache invalidation.
- **Spec §**: 4
- **Ref**: `util.c` (`ovl_set_impure`), `overlayfs.h` (`ovl_is_impuredir`)
- **Lock**: INODE, UPPER

### P2-04 Origin verification on lookup (offline-change detection)
- **What**: when `nfs_export=on` or `index=on`, lookup of a merged dir
  verifies the found lower dir FH/UUID matches `overlay.origin` stored on
  upper. Mismatch → dir not merged (or error).
- **Spec §**: 16, 18
- **Ref**: `namei.c` (`ovl_check_origin`, `ovl_verify_origin_xattr`,
  `ovl_verify_set_fh`)
- **Lock**: VFS, UPPER (xattr read)

### P2-05 POSIX ACL get/set
- **What**: `ovl_get_acl`/`ovl_set_acl`/`do_ovl_get_acl` with idmapping.
  `ovl_get_acl_path` reads from real inode. ACLs are copied up with metadata.
- **Spec §**: 9
- **Ref**: `inode.c` (`ovl_get_acl`, `ovl_set_acl`, `do_ovl_get_acl`,
  `ovl_get_acl_path`, `ovl_idmap_posix_acl`)
- **Lock**: UPPER

### P2-06 fileattr get/set (append/immutable/noatime)
- **What**: `ovl_fileattr_get`/`set` read/write inode flags. Append/immutable
  flags can't be copied up (would block link), so stored in `overlay.protattr`
  xattr. `ovl_check_protattr`/`ovl_set_protattr` round-trip.
- **Spec §**: 6
- **Ref**: `inode.c` (`ovl_fileattr_get`, `ovl_fileattr_set`,
  `ovl_real_fileattr_get`/`set`, `ovl_check_protattr`, `ovl_set_protattr`,
  `ovl_fileattr_prot_flags`)
- **Lock**: INODE, UPPER

### P2-07 nlink preservation (without index)
- **What**: `ovl_get_nlink`/`ovl_set_nlink_upper`/`ovl_set_nlink_lower`:
  when a hardlinked file is copied up without `index`, the link is "broken".
  `overlay.nlink` xattr preserves the original nlink for reporting.
- **Spec §**: 6, 19
- **Ref**: `inode.c` (`ovl_get_nlink`, `ovl_set_nlink_upper`,
  `ovl_set_nlink_lower`), `util.c` (`ovl_nlink_start`/`end`)
- **Lock**: INODE, UPPER

### P2-08 copy_file_range / remap_file_range
- **What**: `ovl_copyfile` dispatches copy/remap with cross-fs fallback.
  Requires both files copied up. (Linux also registers `.clone_file_range`
  via `ovl_copyfile` dispatch internally; no separate `ovl_clone_file_range`
  function exists.)
- **Spec §**: 6
- **Ref**: `file.c` (`ovl_copyfile`, `ovl_copy_file_range`,
  `ovl_remap_file_range`)
- **Lock**: INODE, UPPER

### P2-09 fiemap delegation
- **What**: `ovl_fiemap` delegates to real upper inode (after copy-up).
- **Spec §**: 6
- **Ref**: `inode.c` (`ovl_fiemap`)
- **Lock**: UPPER

### P2-10 file_modified / file_end_write / file_accessed hooks
- **What**: page-cache invalidation hooks after write/end-write. `ovl_file_modified` invalidates lower page cache on copy-up.
- **Spec §**: 6, 19
- **Ref**: `file.c` (`ovl_file_modified`, `ovl_file_end_write`,
  `ovl_file_accessed`)
- **Lock**: NONE (VFS hooks)

### P2-11 UUID modes (null/off/on/auto)
- **What**: `uuid=off`/`null`/`on`/`auto`. `on` generates and stores
  `overlay.uuid` xattr. `auto` upgrades/downgrades. Affects `fsid` reporting.
- **Spec §**: 12
- **Ref**: `super.c`, `util.c` (`ovl_init_uuid_xattr`)
- **Lock**: UPPER (xattr)

### P2-12 fsync_mode (auto/strict)
- **What**: `fsync=strict` adds explicit `fsync` on upper dirs during
  copy-up. `auto` only fsyncs data file. (volatile is P3.)
- **Spec §**: 13
- **Ref**: `copy_up.c`, `file.c` (`ovl_fsync`), `util.c` (`ovl_should_sync`,
  `ovl_should_sync_metadata`)
- **Lock**: UPPER

### P2-13 userxattr namespace
- **What**: `userxattr=on` switches to `user.overlay.*` namespace for
  unprivileged mounts. Affects all xattr read/write in P1-33 and copy-up
  xattr (P1-06).
- **Spec §**: 20
- **Ref**: `xattrs.c` (`ovl_xattr_handlers` selection), `overlayfs.h`
  (`ovl_xattr`, `ovl_xattr_table`)
- **Lock**: NONE (namespace selection)

### P2-14 Nested overlay xattr escaping
- **What**: `overlay.overlay.` prefix escape/unescape for nesting. `ovl_is_escaped_xattr`/`ovl_xattr_escape_name`. Alternative (xattr) whiteout support in lower.
- **Spec §**: 11
- **Ref**: `xattrs.c` (`ovl_is_escaped_xattr`, `ovl_xattr_escape_name`)
- **Lock**: NONE

### P2-15 Layer casefold support
- **What**: if underlying fs supports casefolding, overlay dentries use
  case-insensitive d_ops. `ovl_dentry_casefolded`, `ovl_casefold` in readdir.
- **Spec §**: 2
- **Ref**: `super.c` (`ovl_dentry_ci_operations`), `readdir.c` (`ovl_casefold`)
- **Lock**: NONE

### P2-16 Layer specification via file descriptors
- **What**: new mount API `FSCONFIG_SET_FD` for `lowerdir+`/`datadir+`/
  `upperdir`/`workdir+`. v6.13+.
- **Spec §**: 10
- **Ref**: `params.c`
- **Lock**: NONE

### P2-17 Colon escaping in lowerdir names
- **What**: old API `\:` escaping; new API `lowerdir+` raw + `\072` in
  mountinfo.
- **Spec §**: 10
- **Ref**: `params.c`
- **Lock**: NONE

---

## P3 — Advanced Extensions (out of scope for initial waves)

### P3-01 index feature (hardlink preservation)
- **What**: `index=on` creates index entries (hardlink for non-dir,
  `overlay.upper` xattr for dir) named by hex origin FH. Preserves hardlinks
  across copy-up. Mount-time origin verification of upper root.
- **Spec §**: 16
- **Ref**: `namei.c` (`ovl_lookup_index`, `ovl_get_index_name`,
  `ovl_get_index_fh`, `ovl_verify_index`), `copy_up.c` (`ovl_create_index`,
  `ovl_link_up`), `super.c` (`ovl_get_indexdir`)
- **Lock**: VFS, INODE, IU, UPPER

### P3-02 nfs_export feature
- **What**: `nfs_export=on` (requires `index=on` for rw). `ovl_export_operations`:
  `encode_fh`, `fh_to_dentry`, `fh_to_parent`, `get_parent`, `get_name`.
  Connected vs disconnected dentry handling. Mount-time index verification.
- **Spec §**: 17
- **Ref**: `export.c` (all), `namei.c` (index integration)
- **Lock**: VFS, INODE, UPPER

### P3-03 metacopy (metadata-only copy-up)
- **What**: `metacopy=on` copies metadata only; `overlay.metacopy` xattr marks
  upper file has no data. Data copied on first write-open.
  `lowerdata_redirect` points to data file. Conflicts with some redirect/nfs
  modes.
- **Spec §**: 8
- **Ref**: `copy_up.c` (`ovl_copy_up_metadata`, `ovl_check_metacopy_xattr`,
  `ovl_set_metacopy_xattr`), `file.c` (deferred data copy-up), `namei.c`
  (`ovl_verify_lowerdata`)
- **Lock**: INODE, UPPER

### P3-04 Data-only lower layers
- **What**: `datadir+` / `::` separator. Layers only supply file data for
  metacopy redirects; names/metadata invisible. Implicit metacopy enable.
- **Spec §**: 8
- **Ref**: `super.c` (`ovl_get_layers` data-only handling), `params.c`
- **Lock**: NONE (layer setup)

### P3-05 fs-verity support
- **What**: `verity=off`/`on`/`require`. Digest stored in `overlay.metacopy`
  xattr. `ovl_ensure_verity_loaded`, `ovl_validate_verity`, `ovl_get_verity_digest`.
- **Spec §**: 14
- **Ref**: `copy_up.c` (verity digest), `namei.c`
- **Lock**: UPPER

### P3-06 Volatile mount
- **What**: `fsync=volatile`/`volatile`. Omit all syncs. Create
  `$workdir/work/incompat/volatile` marker. Refuse mount if marker exists.
  errseq-based permanent failure after upper writeback error.
- **Spec §**: 15
- **Ref**: `super.c` (`ovl_create_volatile_dirty`), `file.c` (`ovl_fsync`),
  `util.c` (`ovl_should_sync` returns false), `ovl_fs->errseq`
- **Lock**: UPPER

### P3-07 override_creds (new mount API)
- **What**: `override_creds=on` uses calling task creds as creator creds
  instead of mounter creds. New mount API only (v6.15+).
- **Spec §**: 9, 10
- **Ref**: `super.c` (`ovl_fill_super_creds`), `params.c`
- **Lock**: NONE

### P3-08 Trap inodes (dentry cache traps)
- **What**: `ovl_setup_trap`/`ovl_get_trap_inode`/`ovl_lookup_trap_inode`
  prevent stale dentries from layer mountpoint changes. Used in P0-02 layer
  setup (minimal trap setup); P3-08 covers the full trap lifecycle (creation,
  lookup, cleanup). Traps also detect online layer mountpoint changes per
  Spec §18 (online changes to underlying filesystems while overlay is mounted
  are NOT allowed — behavior is undefined; traps are the detection mechanism).
- **Spec §**: 2, 18
- **Ref**: `super.c` (`ovl_setup_trap`, `ovl_get_trap_inode`), `inode.c`
  (`ovl_lookup_trap_inode`)
- **Lock**: VFS

### P3-09 workdir cleanup on mount
- **What**: `ovl_workdir_cleanup` removes stale temp entries from previous
  crashed mounts. `ovl_indexdir_cleanup` verifies index entries.
- **Spec §**: 6, 17
- **Ref**: `readdir.c` (`ovl_workdir_cleanup`, `ovl_indexdir_cleanup`)
- **Lock**: UPPER

---

## xfstests overlay Test Mapping

This section maps `xfstests/tests/overlay/` test numbers to the micro-features
they validate. The Designer uses this to write validation contracts; the
Checker uses it to select test subsets per pass. Tests requiring features not
yet implemented will `_notrun` automatically (via `_require_scratch_overlay_features`),
so running the full `overlay` group is safe at any implementation stage — only
the tests for implemented features will actually execute.

**Test source**: https://github.com/kdave/xfstests/tree/master/tests/overlay
**Run command**: `./check -overlay -g auto` (or `-g quick` for fast subset)

### P0 — Mandatory Core (read-only mount + stat + readdir)

| Tests | Micro-features covered | Notes |
| :--- | :--- | :--- |
| `001` | P0-01, P0-02, P0-04, P0-05 | Basic mount + stat |
| `002` | P0-04, P0-08, P0-09 | Basic lookup + merged dir |
| `003` | P0-11 | Basic whiteout (char dev 0/0) |
| `004` | P0-08, P0-09, P0-10 | Lookup with opaque/whiteout |
| `005` | P0-08, P0-09, P0-14 | Lookup + readdir dedup |
| `007` | P0-14, P0-15 | Merged dir readdir + d_ino |
| `017` | P0-12, P0-15 | Inode number consistency across copy-up (stat) |
| `019` | P0-12, P0-14 | stat + readdir consistency |
| `021` | P0-02, P0-03, P0-05 | Mount with workdir |
| `035` | P0-02, P0-18 | Read-only mount (no upperdir) |
| `077` | P0-14, P0-15 | Readdir cache invalidation, stale entries |

### P1 — Basic Usability (writable overlay + file ops + permissions)

| Tests | Micro-features covered | Notes |
| :--- | :--- | :--- |
| `006` | P1-02, P1-25, P1-26 | Whiteout after rename (copy-up + whiteout) |
| `008` | P1-22, P1-23, P1-24 | File ownership over whiteout (create-over-whiteout) |
| `009` | P1-04, P1-06 | Copy-up + xattr |
| `010` | P1-25, P1-27 | Remove dir with whiteout from lower |
| `011` | P1-26 | Hardlink over whiteout |
| `012` | P1-26 | Stale upper dentry on unlink |
| `013` | P1-04, P1-21 | Copy-up + create in upper |
| `014` | P1-04, P1-06 | Multi-lower copy-up |
| `015` | P1-22, P1-24, P1-18 | SGID bit inheritance over whiteout (perms) |
| `016` | P1-18, P1-24 | SGID inheritance on create |
| `018` | P1-04, P1-28, P1-31 | Inode/nlink consistency across copy-up + hardlink |
| `020` | P1-24, P1-26, P1-31 | Basic create + unlink + cache invalidation |
| `023` | P1-03, P1-34 | Workdir ACL cleanup on mount |
| `024` | P1-04, P1-07 | Copy-up with origin FH |
| `025` | P1-04, P1-16 | Copy-up + setattr |
| `026` | P1-04, P1-32 | Copy-up symlink |
| `027` | P1-04, P1-21 | Copy-up + create upper |
| `028` | P1-04, P1-28 | Copy-up + hardlink |
| `029` | P1-08, P1-10 | Nested overlay file access (read delegation) |
| `031` | P1-25, P1-26 | Whiteout exposure after remount |
| `032` | P1-29, P1-30 | Rename within same dir |
| `033` | P1-29, P1-30 | Rename + copy-up |
| `034` | P1-29, P1-30 | Rename + whiteout |
| `037` | P1-04, P1-31 | Copy-up + readdir cache invalidation |
| `039` | P1-08, P1-10, P1-12 | mmap of lower file (MAP_SHARED divergence) |
| `040` | P1-08, P1-13 | fsync delegation |
| `078` | P1-18 | Mount option validation (perms) |

### P2 — POSIX Completeness (xino, redirect_dir, ACL, fileattr, nlink)

| Tests | Micro-features covered | Notes |
| :--- | :--- | :--- |
| `017` | P2-02 | redirect_dir (inode number across rename) |
| `030` | P2-06 | immutable/append files (fileattr) |
| `038` | P2-01 | xino d_ino consistency (samefs) |
| `041` | P2-01 | xino d_ino consistency (nonsamefs) |
| `042` | P2-01 | xino st_ino consistency |
| `043` | P2-01, P2-02 | xino + redirect_dir |
| `044` | P2-07 | nlink preservation (without index) |
| `057` | P2-02 | redirect_dir rename |
| `075` | P2-06 | immutable dirs in lower (fileattr) |
| `076` | P2-06 | chattr on overlay dirs (fileattr deadlock) |
| `078` | P2-11, P2-12 | Mount option validation (uuid, fsync) |
| `081` | P2-11 | UUID/fsid modes |
| `083` | P2-13 | userxattr namespace |
| `084` | P2-13, P2-14 | xattr escape + userxattr (nested) |
| `109` | P2-01, P2-13 | unionmount-testsuite (xino, nonsamefs) |

### P3 — Advanced Extensions (index, nfs_export, metacopy, verity, nested)

| Tests | Micro-features covered | Notes |
| :--- | :--- | :--- |
| `022` | P3-08 | Disallow overlay as upperdir (trap) |
| `045`~`048` | P3-09 | fsck.overlay (requires `fsck.overlay` tool) |
| `050`~`055` | P3-01, P3-02 | index + nfs_export (file handle encode/decode) |
| `058`~`064` | P3-01, P3-02, P3-03 | index + nfs_export + metacopy |
| `060`~`064` | P3-03 | metacopy (metadata-only copy-up) |
| `065`~`067` | P3-01 | Mount error cases (index, overlapping layers) |
| `068`~`071` | P3-01, P3-02, P3-08 | Nested overlay + nfs_export + index |
| `073` | P3-01 | Whiteout inode sharing (index) |
| `079` | P3-03, P3-04 | metacopy + data-only layers |
| `080` | P3-03, P3-05 | metacopy + verity |
| `085` | P3-03, P3-04 | metacopy + data-only (lazy follow) |
| `088` | P3-03, P3-05 | metacopy + verity (lazy data) |
| `089` | P3-03, P3-05 | metacopy + verity (I/O error) |
| `111`~`117` | P3-01, P3-08 | unionmount-testsuite nested (index, xino, samefs/nonsamefs) |

### Test Selection Strategy per Wave

- **After P0 wave**: run `overlay/001 002 003 004 005 007 017 019 021 035 077`.
  These validate read-only mount, lookup, readdir, stat, whiteout detection.
- **After P1 wave**: add `overlay/006 008 009 010 011 012 013 014 015 016 018 020 023 024 025 026 027 028 029 031 032 033 034 037 039 040 078`.
  These validate copy-up, create/unlink/rename, permissions, file ops.
- **After P2 wave**: add xino/redirect_dir/fileattr/nlink/userxattr tests.
- **After P3 wave**: add index/nfs_export/metacopy/verity/nested tests.
- Tests requiring unimplemented features will `_notrun` automatically, so
  `./check -overlay -g auto` is safe at any stage — it self-filters.

### unionmount-testsuite (supplementary)

The standalone unionmount-testsuite (https://github.com/amir73il/unionmount-testsuite)
is integrated into xfstests via `overlay/109`~`117`. It provides systematic
correctness verification for core union semantics. Run directly:
`./run --ov --verify`. The xfstests integration parameterizes it with
`--samefs`/`--xino`/`--ovov` (nested) variants.

---

## Coverage Summary

| Tier      |  Count | Milestone                                                                                                                                                                       |
| :-------- | -----: | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| P0        |     18 | Read-only overlay mounts, stats, readdir. Functionally complete; not security-complete (P1-18 must follow).                                                                     |
| P1        |     37 | **Basically usable overlayfs**: mount, read, write, create, delete, rename (EXDEV fallback), two-step permission check, page_cache forwarding. Only optional extensions remain. |
| P2        |     17 | POSIX-completeness: xino, redirect_dir (+redirect_max), ACLs, fileattr, nlink, UUID, strict fsync, userxattr, nesting, casefold, FD layers.                                     |
| P3        |      9 | Advanced: index, nfs_export, metacopy, data-only layers, verity, volatile, override_creds, traps (full lifecycle), workdir cleanup.                                             |
| **Total** | **81** |                                                                                                                                                                                 |

## Notes for the Architect

- This inventory is deliberately flat. Meso-component grouping is the
  Architect's job when building the Bi-Directional Traceability Matrix.
- The P0/P1/P2/P3 tiering is a **suggested** scope ordering, not a hard
  dependency graph. The Architect may reorder within a tier or promote/demote
  items based on Asterinas-specific constraints (e.g., if Asterinas VFS lacks
  idmapping, P2-05 ACL idmapping may be deferred).
- Each micro-feature's `Lock` tag tells the Designer which lock domains are
  involved; the Architect uses these to build the Global Static Lock Topology.
- The `Ref` tag points to the Linux reference file(s); the Architect should
  NOT copy Linux structure but use it to verify spec coverage.
- Pass slicing (main agent) will name explicit subsets of this inventory per
  Creator Pass. The tiering helps the main agent sequence waves: P0 wave(s)
  first, then P1 wave(s), etc.

## Asterinas-Specific Architect Notes (critical divergences from Linux)

These are verified facts about the Asterinas substrate that the Architect
MUST account for in the Global Lock Topology. They are not optional.

1. **VFS does NOT hold a parent-directory lock across inode ops.** Linux
   overlayfs relies on VFS holding `i_rwsem` on the parent directory across
   `lookup`/`create`/`unlink`/`rename`/`rmdir`. Asterinas VFS invokes inode
   ops with NO such lock held (verified: `kernel/src/fs/vfs/fs_apis/inode.rs`
   op signatures take no parent-lock parameter; legacy `overlayfs/fs.rs`
   carries `// TODO: Hold the upper lock from here to avoid race condition`
   comments at `create`/`unlink`). Consequence: the overlay MUST introduce its
   own per-overlay-directory serialization lock (a new `DIR` domain, `Mutex`),
   taken at the entry of every mutating directory inode op. This is a new
   lock domain not present in Linux overlayfs and must be added to the Global
   Lock Topology as the outermost overlay-owned lock. The `VFS` tag in this
   inventory therefore means "parent dir consistency the overlay must
   establish," NOT "VFS-held lock we can rely on."

2. **Only `ostd::sync::Mutex` is a safe sleep lock.** `ostd::sync::RwLock` is
   spin-based (`PreemptDisabled` guard; `ostd/src/sync/rwlock.rs` documents
   itself as "Spin-based Read-write Lock"). Any critical section that may
   trigger BIO (`read_blocks`/`write_blocks`/`VmIo`/`BioWaiter::wait`/page
   cache eviction/any call into upper or lower fs that may sleep) MUST use
   `Mutex`. `INODE`, `CUL`, `UPPER`, and the new `DIR` domain all cross BIO
   and must be `Mutex`. `WL` is a short critical section with no BIO and may
   use `SpinLock` or `Mutex` (prefer `Mutex` for safety).

3. **No reentrant locks.** Any reentrant path (e.g., a VFS callback that
   calls back into the same fs) must release the lock before the reentrant
   call or use different lock granularity. The Architect must identify all
   reentrant paths in the dynamic topology and ensure the lock hierarchy
   permits them.

4. **`page_cache()` may trigger copy-up.** `OverlayInode::page_cache()`
   (P1-37) forwards to the upper inode's page cache, but first triggers
   copy-up if the object is on a lower layer. This means `page_cache()` is a
   BIO-capable call. The Designer must ensure that any caller path that
   holds a lock while invoking `page_cache()` does not violate the hierarchy
   (in particular, the VFS mmap path must not hold a lock that `INODE`/`CUL`
   would re-enter).
