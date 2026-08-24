<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reference Implementation Summary — Linux `fs/overlayfs/`

This prior summarizes the Linux kernel overlayfs reference implementation at
`~/linux/fs/overlayfs/` (kernel tree, `master` branch) plus the authoritative
documentation at `~/linux/Documentation/filesystems/overlayfs.rst`.

It is intended for the **Architect** to internalize the macro/meso/micro
topology and the static lock boundaries, and for the **Designer** to derive
dynamic execution paths. It is NOT a packet; Creators never receive this file.

Source surface: 14 `.c` + 3 `.h` files, ~14.1k lines total.

## 1. Macro Topology (Final Owners)

Linux overlayfs is a pure in-memory stacking filesystem — it owns no on-disk
format of its own. All persistent state is stored in the **upper filesystem**
via regular files, directories, and `trusted.overlay.*` / `user.overlay.*`
extended attributes. The overlay's own superblock, inodes, and dentries are
in-memory VFS objects that delegate to underlying-layer VFS objects.

### Macro-Owner 1: `OverlayFs` (superblock / mount instance)

- **On-disk Structure Owner**: none directly; persistent state is delegated to
  the upper filesystem. The overlay only owns in-memory mount configuration.
- **In-memory carrier**: `struct ovl_fs` (`ovl_entry.h:46`) stored in
  `super_block->s_fs_info`.
- **Key fields**:
  - `layers: struct ovl_layer[]` — ordered layer stack (index 0 = upper).
  - `fs: struct ovl_sb[]` — one entry per unique underlying superblock.
  - `workbasedir`, `workdir: struct dentry *` — work directory dentries.
  - `config: struct ovl_config` — mount options (upperdir, workdir, lowerdirs,
    default_permissions, redirect_mode, verity_mode, index, uuid, nfs_export,
    xino, metacopy, userxattr, fsync_mode).
  - `creator_cred: const struct cred *` — stashed credentials for underlying
    access (see Permission Model below).
  - `xino_mode: int` — -1 disabled, 0 same fs, 1..32 unused ino bits.
  - `last_ino: atomic_long_t` — non-persistent inode number generator.
  - `whiteout: struct dentry *` + `whiteout_lock: struct mutex` — shared
    whiteout cache.
  - `errseq: errseq_t` — r/o snapshot of upperdir sb errseq for volatile mounts.
- **Lifecycle**: `ovl_fill_super` (`super.c:1541`) is the entry point invoked by
  the new mount API. It calls `ovl_fill_super_creds` → `ovl_get_upper` →
  `ovl_make_workdir` → `ovl_get_indexdir` → `ovl_get_layers` →
  `ovl_get_lowerstack` → `ovl_get_root` to assemble the layer stack and root
  dentry. `ovl_put_super` (`super.c:225`) tears down. `ovl_sync_fs`
  (`super.c:234`) forwards sync to the upper filesystem.

### Macro-Owner 2: `OverlayInode` (overlay VFS inode)

- **On-disk Structure Owner**: none; the overlay inode is an in-memory facade.
  Persistent metadata lives on the upper-layer dentry (post copy-up) or the
  lower-layer dentry (pre copy-up).
- **In-memory carrier**: `struct ovl_inode` (`ovl_entry.h:131`), embedded
  inside the VFS `struct inode` via `container_of` (`OVL_I(inode)`).
- **Key fields**:
  - `__upperdentry: struct dentry *` — the upper-layer dentry (NULL for
    lower-only objects). Accessed via `READ_ONCE` (`ovl_upperdentry_dereference`).
  - `oe: struct ovl_entry *` — the lower stack (see Macro-Owner 3).
  - `redirect: const char *` — directory redirect path (for `redirect_dir`).
  - `cache` / `lowerdata_redirect`: union — directory readdir cache OR regular
    file lower-data redirect path (metacopy).
  - `flags: unsigned long` — `OVL_IMPURE`, `OVL_WHITEOUTS`, `OVL_INDEX`,
    `OVL_UPPERDATA`, `OVL_CONST_INO`, `OVL_HAS_DIGEST`, `OVL_VERIFIED_DIGEST`.
  - `version: u64` — directory cache version for invalidation.
  - `lock: struct mutex` — serializes copy up and metadata updates
    (`ovl_inode_lock` / `ovl_inode_unlock`).
- **VFS inode ops**: `ovl_file_inode_operations` (`inode.c:729`),
  `ovl_symlink_inode_operations` (`inode.c:743`),
  `ovl_special_inode_operations` (`inode.c:751`).
- **Inode allocation**: `ovl_alloc_inode` (`super.c:184`) uses a dedicated
  `kmem_cache` (`ovl_inode_cachep`). `ovl_new_inode` / `ovl_get_inode`
  (`inode.c`) construct inodes from `struct ovl_inode_params` (upperdentry, oe,
  index, redirect, lowerdata_redirect).

### Macro-Owner 3: `OverlayEntry` (per-dentry lower stack)

- **On-disk Structure Owner**: none; in-memory only.
- **In-memory carrier**: `struct ovl_entry` (`ovl_entry.h:71`), a flexible
    array struct holding `__numlower` `struct ovl_path` entries.
- **`struct ovl_path`** (`ovl_entry.h:65`): `{ layer, dentry }` — pins a lower
  dentry and the layer it belongs to.
- **Per-dentry flags** (`enum ovl_entry_flag`): `OVL_E_UPPER_ALIAS`,
  `OVL_E_OPAQUE`, `OVL_E_CONNECTED`, `OVL_E_XWHITEOUTS`. Stored in
  `dentry->d_fsdata`.
- **Lifecycle**: `ovl_alloc_entry` / `ovl_free_entry` (`util.c`). Attached to
  the overlay dentry at lookup time (`ovl_lookup` in `namei.c`).

### Macro-Owner 4: `OverlayLayer` (layer descriptor)

- **In-memory carrier**: `struct ovl_layer` (`ovl_entry.h:23`).
- **Key fields**: `mnt: struct vfsmount *` (MUST be first member for
  `ovl_free_fs`), `trap: struct inode *` (dentry cache trap), `fs: struct ovl_sb
  *`, `idx: int` (0 = upper), `fsid: int` (0 = upper fs), `has_xwhiteouts: bool`.
- **`struct ovl_sb`** (`ovl_entry.h:13`): `{ sb, pseudo_dev, bad_uuid, is_lower
  }` — one entry per unique underlying superblock.

## 2. Meso-Component Map (file → responsibility)

| File          | Lines | Meso-Component      | Responsibility                                                                    |
| :------------ | ----: | :------------------ | :-------------------------------------------------------------------------------- |
| `super.c`     |  1622 | Mount & Superblock  | Layer setup, workdir/indexdir creation, superblock ops, inode cache, `fill_super` |
| `params.c`    |  1104 | Mount Options       | `fs_parameter_spec` parsing for all mount options, config defaults                |
| `namei.c`     |  1483 | Path Lookup         | `ovl_lookup`, redirect following, origin/upper FH verification, index lookup      |
| `dir.c`       |  1496 | Directory Mutations | create/mkdir/mknod/symlink/link/unlink/rmdir/rename, whiteout, opaque, redirect   |
| `readdir.c`   |  1326 | Readdir             | Merged-directory readdir cache, whiteout enumeration, d_type, ino remap           |
| `inode.c`     |  1298 | Inode Attributes    | getattr/setattr/permission/get_link/acl/update_time/fileattr, inode construction  |
| `file.c`      |   652 | File Operations     | open/release/read_iter/write_iter/llseek/mmap/fsync/fallocate/copy_file_range     |
| `copy_up.c`   |  1291 | Copy Up             | Full & metadata-only copy up, tmpfile, workdir staging, origin FH, index creation |
| `export.c`    |   868 | NFS Export          | encode/decode file handles, `fh_to_dentry`, `get_parent`, connectable layer       |
| `util.c`      |  1518 | Helpers             | Path-type computation, layer/dentry accessors, stack alloc, creds, write access   |
| `xattrs.c`    |   261 | Xattr Handlers      | `ovl_xattr_handlers`, escape/own/private xattr classification                     |
| `overlayfs.h` |   954 | Shared Declarations | Enums, inline accessors, `ovl_do_*` VFS-call wrappers                             |
| `ovl_entry.h` |   193 | Core Structures     | `ovl_fs`, `ovl_inode`, `ovl_entry`, `ovl_layer`, `ovl_path`, `ovl_config`         |
| `params.h`    |    44 | Params Header       | `ovl_apply_options` prototype                                                     |

## 3. Micro-Feature Inventory (per meso-component)

### 3.1 Mount & Superblock (`super.c`)
- `ovl_fill_super` / `ovl_fill_super_creds`: entry point, stashes creator creds.
- `ovl_get_upper`: resolves upperdir, takes `upperdir_locked`, sets up trap.
- `ovl_make_workdir`: creates/verifies workdir on same fs as upperdir, checks
  `d_type` support, volatile dirty marker, xwhiteout capability.
- `ovl_get_indexdir`: creates index dir for `index=` / `nfs_export=`.
- `ovl_get_layers`: assembles `ovl_layer[]` from lowerdir paths, assigns fsid,
  checks UUID, sets up per-layer traps.
- `ovl_get_lowerstack`: builds root `ovl_entry` lower stack.
- `ovl_get_root`: constructs root dentry/inode, sets `d_op`.
- `ovl_put_super` / `ovl_sync_fs` / `ovl_statfs`: superblock ops.
- `ovl_alloc_inode` / `ovl_free_inode` / `ovl_destroy_inode`: inode cache.
- `ovl_dentry_revalidate` / `ovl_dentry_weak_revalidate`: dentry ops (layer
  mountpoint change detection).

### 3.2 Mount Options (`params.c`)
- Parameter spec: `lowerdir`, `lowerdir+`, `upperdir`, `workdir`, `workdir+`,
  `datadir+`, `default_permissions`, `redirect_dir`, `index`, `uuid`, `nfs_export`,
  `xino`, `metacopy`, `userxattr`, `fsync`, `verity`, `override_creds`.
- Config defaults driven by `CONFIG_OVERLAY_FS_*` Kconfig.
- `ovl_parse_param` / `ovl_parse_param_lowerdir` / `ovl_apply_options` /
  `ovl_free_options`: option parsing and application.

### 3.3 Path Lookup (`namei.c`)
- `ovl_lookup`: the core lookup entry. Iterates layers top-down, follows
  redirects, checks opaque, builds `ovl_entry` stack, resolves index.
- `ovl_lookup_single` / `ovl_lookup_layer`: per-layer lookup with redirect and
  opaque handling.
- `ovl_check_redirect` / `ovl_get_redirect_xattr`: redirect xattr parsing.
- `ovl_check_origin` / `ovl_verify_origin_xattr` / `ovl_verify_set_fh`: origin
  FH verification (NFS export / index).
- `ovl_lookup_index` / `ovl_get_index_name` / `ovl_get_index_fh`: index dir
  lookup by FH.
- `ovl_decode_real_fh` / `ovl_uuid_match`: FH decoding for export.
- `ovl_path_next`: iterate stack entries by index.

### 3.4 Directory Mutations (`dir.c`)
- `ovl_create` / `ovl_mkdir` / `ovl_mknod` / `ovl_symlink` / `ovl_link` /
  `ovl_rename` / `ovl_unlink` / `ovl_rmdir`: `inode_operations` for directories.
- `ovl_create_object` / `ovl_create_or_link`: shared create path, decides
  upper-only vs over-whiteout.
- `ovl_create_upper`: create directly in upper.
- `ovl_create_over_whiteout`: create over a whiteout (atomic-rename over
  prepared temp).
- `ovl_clear_empty`: handle directory rename that empties a merged dir.
- `ovl_set_redirect`: set redirect xattr on directory rename.
- `ovl_cleanup_and_whiteout` / `ovl_whiteout`: whiteout creation (char dev 0/0
  or xattr whiteout).
- `ovl_set_opaque` / `ovl_set_opaque_xerr`: set opaque xattr.
- `ovl_create_real` / `ovl_create_temp` / `ovl_cleanup`: workdir temp helpers.
- `ovl_cleanup_handle_whiteouts`: handle xwhiteouts during create.

### 3.5 Readdir (`readdir.c`)
- `ovl_dir_operations`: file_operations for overlay directories.
- `ovl_readdir` / `ovl_iterate`: merged readdir entry.
- `ovl_cache_get`: build/refresh the per-directory `ovl_dir_cache` (rb-tree +
  list). Cached in `ovl_inode->cache`, version-checked against `ovl_inode->version`.
- `ovl_dir_read_merged`: read upper first, then lower layers, dedup by name.
- `ovl_fill_merge`: per-entry fill callback, dedup, d_ino computation.
- `ovl_check_whiteouts` / `ovl_cleanup_whiteouts`: enumerate and clean
  xwhiteouts.
- `ovl_calc_d_ino` / `ovl_remap_lower_ino`: d_ino / st_ino remapping for xino.
- `ovl_dir_real_file`: get the underlying real file for the directory.
- `ovl_check_empty_dir`: used by rmdir to verify emptiness (respecting
  whiteouts).
- `ovl_workdir_cleanup` / `ovl_indexdir_cleanup`: mount-time cleanup of stale
  workdir/index entries.

### 3.6 Inode Attributes (`inode.c`)
- `ovl_getattr` / `ovl_statfs`: stat with dev/ino remapping (xino, fsid).
- `ovl_setattr`: chmod/chown/utimes → triggers copy up then forwards to upper.
- `ovl_permission`: two-step permission check (local DAC + stashed creds on
  real underlying inode).
- `ovl_get_link`: symlink read (copy up target if needed).
- `ovl_get_acl` / `ovl_set_acl` / `do_ovl_get_acl`: POSIX ACL with idmapping.
- `ovl_update_time`: atime/mtime/ctime updates (copy up then forward).
- `ovl_fileattr_get` / `ovl_fileattr_set`: fileattr (append/immutable/noatime)
  with protattr xattr for flags that block copy-up.
- `ovl_check_protattr` / `ovl_set_protattr`: protattr xattr round-trip.
- `ovl_map_dev_ino`: dev/ino remapping core.
- `ovl_copyattr`: copy real inode attrs to overlay inode.
- `ovl_new_inode` / `ovl_get_inode` / `ovl_lookup_inode`: inode construction.
- `ovl_get_trap_inode` / `ovl_lookup_trap_inode`: dentry cache traps.

### 3.7 File Operations (`file.c`)
- `ovl_file_operations`: file_operations for overlay regular files.
- `ovl_open`: decides real file (upper if copied up, else lower), handles
  copy-up on write flags, allocates `struct ovl_file` wrapper.
- `ovl_release`: drops real file refs, frees `ovl_file`.
- `ovl_read_iter` / `ovl_write_iter`: delegate to real file with creds override.
- `ovl_llseek`: delegate, with copy-up awareness.
- `ovl_mmap`: delegate, requires upper file for shared writable maps.
- `ovl_fsync`: delegate to real file, honor `fsync_mode`.
- `ovl_fallocate` / `ovl_fadvise`: delegate with copy-up.
- `ovl_splice_read` / `ovl_splice_write`: splice delegation.
- `ovl_copyfile` / `ovl_copy_file_range` / `ovl_remap_file_range` /
  `ovl_clone_file_range`: copy/remap delegation with cross-fs fallback.
- `ovl_file_modified` / `ovl_file_end_write` / `ovl_file_accessed`: page-cache
  invalidation hooks.

### 3.8 Copy Up (`copy_up.c`)
- `ovl_copy_up` / `ovl_copy_up_with_data` / `ovl_maybe_copy_up`: entry points.
- `ovl_copy_up_workdir`: full copy up via workdir temp + atomic rename.
- `ovl_copy_up_tmpfile`: copy up via `O_TMPFILE` (when supported).
- `ovl_copy_up_metadata`: copy attrs, xattrs, origin FH, metacopy xattr.
- `ovl_copy_up_data`: copy file data (full copy up).
- `ovl_copy_xattr`: copy xattrs (with `ovl_must_copy_xattr` filter, ACL copy).
- `ovl_set_attr` / `ovl_set_timestamps` / `ovl_set_size`: attribute application.
- `ovl_encode_real_fh` / `ovl_get_origin_fh` / `ovl_set_origin_fh`: origin FH
  encode/store.
- `ovl_create_index`: create index entry for hardlink/NFS export.
- `ovl_link_up`: hardlink to existing upper (index case).
- `ovl_check_metacopy_xattr` / `ovl_set_metacopy_xattr`: metacopy xattr.
- `ovl_copy_up_start` / `ovl_copy_up_end` / `ovl_already_copied_up`: copy-up
  coordination (in `util.c`).

### 3.9 NFS Export (`export.c`)
- `ovl_export_operations` / `ovl_export_fid_operations`.
- `ovl_encode_fh`: encode overlay FH (lower or upper).
- `ovl_fh_to_dentry` / `ovl_fh_to_parent`: decode FH to dentry.
- `ovl_obtain_alias`: construct disconnected dentry alias.
- `ovl_lookup_real` / `ovl_lookup_real_ancestor` / `ovl_lookup_real_inode`:
  connected lookup from decoded FH.
- `ovl_get_name` / `ovl_get_parent`: connectable export ops.
- `ovl_encode_maybe_copy_up` / `ovl_connect_layer` / `ovl_connectable_layer`:
  encode-time copy up for middle-layer redirects.

### 3.10 Helpers (`util.c`)
- Path-type: `ovl_path_type`, `ovl_path_upper`, `ovl_path_lower`,
  `ovl_path_lowerdata`, `ovl_path_real`, `ovl_path_realdata`.
- Dentry accessors: `ovl_dentry_upper`, `ovl_dentry_lower`,
  `ovl_dentry_lowerdata`, `ovl_dentry_real`, `ovl_dentry_set_lowerdata`.
- Inode accessors: `ovl_inode_upper`, `ovl_inode_lower`, `ovl_inode_lowerdata`,
  `ovl_inode_real`, `ovl_inode_realdata`.
- Flag ops: `ovl_dentry_set_flag`/`clear_flag`/`test_flag`/`is_opaque`/
  `is_whiteout`/`has_xwhiteouts`/`has_upper_alias`.
- Stack alloc: `ovl_stack_alloc`/`cpy`/`put`/`free`, `ovl_alloc_entry`/`free_entry`.
- Creds: `ovl_override_creds`, `ovl_creds`, `with_ovl_creds` macro.
- Write access: `ovl_get_write_access`/`put_write_access`, `ovl_want_write`/
  `drop_write`, `ovl_start_write`/`end_write`.
- Inuse lock: `ovl_inuse_trylock`/`unlock`/`is_inuse`.
- Nlink: `ovl_nlink_start`/`end`, `ovl_need_index`.
- Whiteout: `ovl_is_whiteout`, `ovl_path_is_whiteout`, `ovl_workdir`.
- Dir cache: `ovl_dir_cache`, `ovl_set_dir_cache`, `ovl_dir_modified`.
- UUID: `ovl_init_uuid_xattr`, `ovl_can_decode_fh`.

### 3.11 Xattr Handlers (`xattrs.c`)
- `ovl_xattr_handlers`: handler table (trusted / user / other).
- `ovl_is_private_xattr`: classify overlay-private xattrs.
- `ovl_is_escaped_xattr` / `ovl_xattr_escape_name`: nested-overlay escape.
- `ovl_listxattr`: filter private xattrs from listing.
- `ovl_own_xattr_get`/`set` (trusted + user), `ovl_other_xattr_get`/`set`.

## 4. On-Disk Persistent State (via upper filesystem)

Overlayfs stores all persistent state in the upper filesystem using regular
files/dirs plus `trusted.overlay.*` (or `user.overlay.*` with `userxattr`)
extended attributes. The xattr namespace is defined by `enum ovl_xattr`
(`overlayfs.h:43`):

| Xattr                        | Constant              | Purpose                                      |
| :--------------------------- | :-------------------- | :------------------------------------------- |
| `trusted.overlay.opaque`     | `OVL_XATTR_OPAQUE`    | Mark dir opaque ("y") or xwhiteout ("x")     |
| `trusted.overlay.redirect`   | `OVL_XATTR_REDIRECT`  | Directory redirect path                      |
| `trusted.overlay.origin`     | `OVL_XATTR_ORIGIN`    | Copy-up origin FH (lower inode)              |
| `trusted.overlay.impure`     | `OVL_XATTR_IMPURE`    | Upper dir may contain non-pure-upper entries |
| `trusted.overlay.nlink`      | `OVL_XATTR_NLINK`     | Preserved nlink for hardlinks w/o index      |
| `trusted.overlay.upper`      | `OVL_XATTR_UPPER`     | Upper FH stored on index entry               |
| `trusted.overlay.uuid`       | `OVL_XATTR_UUID`      | Overlay instance UUID                        |
| `trusted.overlay.metacopy`   | `OVL_XATTR_METACOPY`  | Metacopy header + optional verity digest     |
| `trusted.overlay.protattr`   | `OVL_XATTR_PROTATTR`  | Protected fileattr (append/immutable)        |
| `trusted.overlay.whiteout`   | `OVL_XATTR_XWHITEOUT` | xattr-based whiteout marker                  |
| `trusted.overlay.xwhiteouts` | (dir xattr)           | Dir contains xwhiteout entries               |

**Whiteout forms**:
1. Char device with `rdev == 0` (classic whiteout).
2. Zero-size regular file with `trusted.overlay.whiteout` xattr (xwhiteout,
   used for nested overlayfs and container-built layers).

**Origin FH format** (`struct ovl_fb` / `struct ovl_fh`,
`overlayfs.h:140-170`): `{ version, magic=0xfb, len, flags, type, uuid, fid[] }`.
Used in `OVL_XATTR_ORIGIN` and `OVL_XATTR_UPPER`.

**Metacopy format** (`struct ovl_metacopy`, `overlayfs.h:175`):
`{ version, len, flags, digest_algo, digest[] }`. Stored in
`OVL_XATTR_METACOPY`.

## 5. Static Lock Topology

Linux overlayfs uses a layered lock discipline. The Designer MUST preserve
this ordering to avoid deadlocks.

### 5.1 Lock Primitives in Use
- `struct mutex` — sleep mutex. Used for: `ovl_inode->lock` (copy up /
  metadata), `ovl_fs->whiteout_lock` (shared whiteout cache), directory
  `i_rwsem` (VFS-provided), upper-layer `i_rwsem` (via VFS calls).
- `rwsem` (`i_rwsem`) — VFS directory lock, held by VFS during lookup/readdir/
  create/rename. Overlayfs relies on VFS to hold parent dir locks.
- `atomic_long_t` / `atomic_t` / `READ_ONCE` / `WRITE_ONCE` — for
  `last_ino`, `__upperdentry`, `flags`, `oe->__numlower`-adjacent fields.
- `errseq_t` — volatile mount errseq snapshot.

### 5.2 Static Lock Hierarchy (outermost → innermost)
1. **VFS-level `i_rwsem`** on overlay parent directory — held by VFS across
   `ovl_lookup` / `ovl_create*` / `ovl_rename` / `ovl_unlink` / `ovl_rmdir`.
   Overlayfs MUST NOT acquire it explicitly; it relies on VFS.
2. **`ovl_inode->lock`** (`mutex`) — serializes copy up and metadata updates
   on a single overlay inode. Acquired by `ovl_inode_lock`/`unlock`. Held
   across `ovl_copy_up*`, `ovl_set_attr`, `ovl_update_time`, nlink updates.
   MUST be released before calling VFS ops that may acquire upper `i_rwsem`
   in a different order.
3. **`ovl_fs->whiteout_lock`** (`mutex`) — guards the shared whiteout dentry
   cache. Short critical section.
4. **Upper-layer `i_rwsem`** — acquired implicitly via `vfs_*` calls
   (`ovl_do_create`, `ovl_do_unlink`, `ovl_do_rename`, etc.) in `overlayfs.h`.
   These are the innermost locks; overlayfs never holds an overlay-level lock
   across them except `ovl_inode->lock` where copy-up semantics require it.
5. **`ovl_inuse_trylock`** — dentry-level in-use lock (uses `d_fsdata` bit)
   to prevent two overlays from using the same upper/workdir. Taken at mount
   and during copy-up coordination.

### 5.3 Lock-Ordering Rules
- **Never hold `ovl_inode->lock` across blocking upper VFS calls that may
  re-enter overlayfs** (e.g., `vfs_*` on an upper path that is itself an
  overlay). The `ovl_do_*` wrappers in `overlayfs.h` are the only sanctioned
  entry to upper VFS.
- **Copy-up coordination**: `ovl_copy_up_start`/`end` use a per-dentry bit
  lock (`d_fsdata`) so only one task copies up a given dentry; others wait
  via `ovl_already_copied_up`. This is OUTSIDE `ovl_inode->lock`.
- **Directory redirect rename**: holds overlay parent `i_rwsem` (VFS) +
  `ovl_inode->lock` of the victim + upper `i_rwsem` of source and target
  parents (via `ovl_do_rename`). The `ovl_clear_empty` path is the most
  lock-intensive sequence in the implementation.
- **Readdir cache**: `ovl_dir_cache` is protected by `ovl_inode->lock` for
  cache swap; the rb-tree itself is built under overlay dir `i_rwsem` (VFS)
  and the per-file `struct file` refcount.
- **No spinlocks**: overlayfs uses no raw spinlocks of its own; all
  synchronization is mutex/rwsem/atomic based. This is consistent with the
  Asterinas `ASTERINAS_INTEGRATION_PRIORS.md` rule that spinlocks forbid
  blocking I/O — overlayfs always sleeps into the upper fs.

## 6. Permission Model (two-step check)

Per `overlayfs.rst` "Permission model" and `inode.c:ovl_permission`:

1. **Local check (a)**: standard DAC + MAC on the overlay inode using the
   *current task* credentials. Ensures consistency before/after copy up
   (owner/group/mode/acls are copied up).
2. **Real check (b)**: check the *stashed* `creator_cred` against the real
   underlying upper/lower inode. Ensures the mount creator does not gain
   privileges the stashed creds lack.

`ovl_override_creds` / `with_ovl_creds(sb)` (`util.c:65`) temporarily switch
to `creator_cred` for underlying VFS calls that do their own permission
checks (e.g., `vfs_getxattr`).

## 7. Copy-Up Semantics

- **Trigger**: first write-open, metadata change (chmod/chown/utime), hardlink,
  rename of a lower/merged object, xattr set on a lower object.
- **Full copy up** (`ovl_copy_up_workdir` / `ovl_copy_up_tmpfile`):
  1. Ensure parent dir is copied up (recursive).
  2. Create temp inode in workdir (or `O_TMPFILE`).
  3. Copy metadata (owner, mode, timestamps, xattrs, origin FH).
  4. Copy data (for regular files, unless metacopy).
  5. `fsync` upper temp (per `fsync_mode`).
  6. Atomically rename temp into upper dir.
  7. Set `OVL_XATTR_ORIGIN` on new upper dentry.
  8. Update overlay inode `__upperdentry` (WRITE_ONCE).
- **Metacopy** (`ovl_copy_up_metadata` only): stores `OVL_XATTR_METACOPY` +
  optional verity digest; data is fetched from lower on first write-open
  (`ovl_copy_up_data` deferred). `lowerdata_redirect` xattr points to the
  data file.
- **Index**: when `index=on` or `nfs_export=on`, a hardlink (non-dir) or
  `OVL_XATTR_UPPER`-bearing entry (dir) is created in the index dir, named
  by the hex origin FH. Used for hardlink preservation and NFS export.
- **Already-copied-up fast path**: `ovl_already_copied_up(dentry, flags)`
  checks `OVL_UPPERDATA` / upper dentry presence before taking the lock.

## 8. Lookup Algorithm (`ovl_lookup`)

1. Allocate `ovl_lookup_data` with the name.
2. For each layer from top (upper) to bottom (lower):
   - `ovl_lookup_layer` → `ovl_lookup_single`: lookup_one on the layer's
     dentry.
   - If found dir and not opaque, continue to next layer (merge).
   - If found non-dir, stop (lower layers hidden).
   - If whiteout found, stop and mark entry as whiteout.
3. Follow redirect xattr if present (`ovl_check_redirect`).
4. If `index=on` and upper exists, `ovl_lookup_index` to find/verify the index
   entry and detect aliasing.
5. Verify origin FH if `nfs_export=on` (`ovl_verify_origin`).
6. Build `ovl_entry` lower stack, allocate overlay inode via `ovl_get_inode`,
   set dentry flags (`OVL_E_UPPER_ALIAS`, `OVL_E_OPAQUE`, etc.).

## 9. Readdir Algorithm (`ovl_readdir`)

1. `ovl_cache_get(dentry)`: if `ovl_inode->cache` is valid (version matches
   `ovl_inode->version`), reuse.
2. Otherwise, allocate new `ovl_dir_cache` (rb-tree + list).
3. `ovl_dir_read_merged`:
   - Read upper dir first (`ovl_dir_read` on upper real file).
   - For each lower layer (top to bottom), read and dedup by name (rb-tree
     lookup). Skip entries hidden by whiteout or opaque.
   - For xwhiteout dirs, enumerate whiteouts to mark entries.
4. Assign sequential seek offsets.
5. Swap cache into `ovl_inode->cache` under `ovl_inode->lock`, bump version.
6. `ovl_iterate` walks the cached list, computing `d_ino` via
   `ovl_calc_d_ino` / `ovl_remap_lower_ino` (xino).
7. `seekdir(0)` discards the cache; `ovl_dir_reset` rebuilds.

## 10. Mount Options Summary

| Option                   | Values                 | Effect                                            |
| :----------------------- | :--------------------- | :------------------------------------------------ |
| `lowerdir` / `lowerdir+` | path                   | Lower layers (colon-separated for old API)        |
| `upperdir`               | path                   | Writable upper layer (omit for read-only)         |
| `workdir` / `workdir+`   | path                   | Work dir, same fs as upperdir                     |
| `datadir+`               | path                   | Data-only lower layers (metacopy)                 |
| `default_permissions`    | bool                   | Use kernel permission check only                  |
| `redirect_dir`           | on/follow/nofollow/off | Directory redirect feature                        |
| `index`                  | bool                   | Hardlink preservation via index dir               |
| `uuid`                   | null/off/on/auto       | Overlay UUID / fsid source                        |
| `nfs_export`             | bool                   | NFS export support (requires index)               |
| `xino`                   | off/auto/on            | Unified st_ino/st_dev via xino bits               |
| `metacopy`               | bool                   | Metadata-only copy up                             |
| `userxattr`              | bool                   | Use `user.overlay.*` namespace                    |
| `fsync`                  | volatile/auto/strict   | Durability during copy up                         |
| `verity`                 | off/on/require         | fs-verity metacopy digest                         |
| `override_creds`         | bool                   | Use caller creds as creator creds (new mount API) |

## 11. Behavioral Notes Relevant to Refactor

- **No on-disk format**: overlayfs is a pure stacker. The "On-disk Structure
  Owner" concept maps to the upper-layer dentry + xattr pair, not a custom
  disk region. The Architect should treat xattr schemas (section 4) as the
  durable contract.
- **Hybrid st_dev/st_ino**: non-dir objects may report the underlying fs
  `st_dev`; `xino` unifies them. The Designer must decide whether the
  Asterinas refactor implements xino or reports overlay-internal dev/ino
  uniformly (simpler, less POSIX-compliant).
- **Copy-up is the central state transition**: every mutation of a lower
  object funnels through copy-up. The Designer should model copy-up as a
  single meso-component with explicit micro-features (full, metacopy,
  tmpfile, index, origin FH).
- **VFS delegation**: overlayfs is thin — most file ops are pure delegation
  to the real upper/lower file via `ovl_real_file`. The Designer should
  preserve this thin-wrapper property and avoid re-implementing VFS logic.
- **NFS export / index / redirect / metacopy / verity** are advanced
  features that significantly complicate the implementation. The Architect
  should decide which features are in scope for the Asterinas refactor;
  a minimal viable overlayfs can omit `index`, `nfs_export`, `metacopy`,
  `verity`, and `redirect_dir` (returning `EXDEV` on cross-dir rename).
- **Permission model**: the two-step check is mandatory for security. The
  Designer must not collapse it into a single check.
- **No spinlocks**: overlayfs uses only mutex/rwsem/atomic. This aligns with
  the Asterinas constraint that spinlocks forbid blocking I/O.

## 12. Source File Inventory (for Architect cross-reference)

| File            | Path                                              | Lines |
| :-------------- | :------------------------------------------------ | ----: |
| `ovl_entry.h`   | `~/linux/fs/overlayfs/ovl_entry.h`                |   193 |
| `overlayfs.h`   | `~/linux/fs/overlayfs/overlayfs.h`                |   954 |
| `params.h`      | `~/linux/fs/overlayfs/params.h`                   |    44 |
| `super.c`       | `~/linux/fs/overlayfs/super.c`                    |  1622 |
| `params.c`      | `~/linux/fs/overlayfs/params.c`                   |  1104 |
| `namei.c`       | `~/linux/fs/overlayfs/namei.c`                    |  1483 |
| `dir.c`         | `~/linux/fs/overlayfs/dir.c`                      |  1496 |
| `readdir.c`     | `~/linux/fs/overlayfs/readdir.c`                  |  1326 |
| `inode.c`       | `~/linux/fs/overlayfs/inode.c`                    |  1298 |
| `file.c`        | `~/linux/fs/overlayfs/file.c`                     |   652 |
| `copy_up.c`     | `~/linux/fs/overlayfs/copy_up.c`                  |  1291 |
| `export.c`      | `~/linux/fs/overlayfs/export.c`                   |   868 |
| `util.c`        | `~/linux/fs/overlayfs/util.c`                     |  1518 |
| `xattrs.c`      | `~/linux/fs/overlayfs/xattrs.c`                   |   261 |
| `overlayfs.rst` | `~/linux/Documentation/filesystems/overlayfs.rst` |   883 |
