<!-- SPDX-License-Identifier: MPL-2.0 -->

# Filesystem Spec Summary — Overlayfs

This prior captures the **authoritative specification** of the overlay filesystem
as documented in `~/linux/Documentation/filesystems/overlayfs.rst` (883 lines).
It is specification-oriented: it states what overlayfs IS and what rules it MUST
obey, not how any particular implementation achieves them. Implementation
mechanics belong in `REFERENCE_IMPLEMENTATION_SUMMARY.md`.

The Architect internalizes this file to build the Bi-Directional Traceability
Matrix; the Designer derives dynamic paths and lock contracts from it. Creators
never receive this file.

## 1. Definition

Overlayfs is a **hybrid stacking filesystem** that presents a unified view of
one **upper** directory tree overlaid on one or more **lower** directory trees.
When a name exists in both upper and lower:
- For non-directories: the upper object is visible; the lower object is hidden.
- For directories: a **merged directory** is formed (name lists are unioned).

The overlay owns no on-disk format. All persistent state is stored in the upper
filesystem via regular files/directories plus `trusted.overlay.*` (or
`user.overlay.*` with `userxattr`) extended attributes.

## 2. Layer Model

- **Upper layer** (optional): writable. If absent, the overlay is read-only.
  Must support creation of `trusted.*` / `user.*` xattrs and must return valid
  `d_type` in readdir. NFS is NOT suitable as upper.
- **Lower layers** (one or more): read-only. Any mountable filesystem qualifies.
  May themselves be overlayfs mounts (nesting, see §11).
- **Layer ordering**: lower layers are stacked from the **topmost** (rightmost
  in the mount option list) to the **bottommost** (leftmost). Lookup searches
  upper first, then lower layers top-to-bottom.
- **Data-only lower layers** (metacopy only, see §8): separated by `::`, may only
  supply file data, never visible names or metadata. A normal lower layer may
  not appear below a data-only layer.
- **Same-filesystem special case**: when all layers share one underlying
  filesystem, `st_dev` is uniform and `st_ino` matches the underlying inode.

## 3. Inode Identity (`st_dev` / `st_ino` / `d_ino`)

| Configuration                     | Persistent `st_ino` | Uniform `st_dev` | `st_ino == d_ino` (dir/!dir) | `d_ino == i_ino` (dir/!dir) |
| :-------------------------------- | :-----------------: | :--------------: | :--------------------------: | :-------------------------: |
| All layers on same fs             |          Y          |        Y         |            Y / Y             |            Y / Y            |
| Layers not on same fs, `xino=off` |          N          |        N         |            N / Y             |            N / Y            |
| `xino=on`/`auto`                  |          Y          |        Y         |            Y / Y             |            Y / Y            |
| `xino=on`/`auto`, ino overflow    |          N          |        N         |            N / Y             |            N / Y            |

- **`xino`**: composes a unique `st_ino` from the real inode number + an `fsid`
  encoded into the high inode bits. Requires the underlying fs to rarely use
  those high bits; falls back to non-xino behavior on overflow.
- **`xino=auto`**: enables xino only if all underlying filesystems support NFS
  file handles (giving persistent `st_ino`).
- Non-directory objects may report the underlying fs `st_dev` unless xino or
  same-fs makes it uniform. Directories always report the overlay `st_dev`.

## 4. Directories — Merging Rules

- **Merged directory**: formed when both upper and lower objects at a name are
  directories. Lookup results from both are cached in the overlay dentry.
- **Non-merged directory**: if either side is a non-directory, only the upper
  object (or the lower object if no upper) is stored; the lower directory is
  hidden.
- **Metadata of merged dir**: reported from the **upper** directory only. Lower
  directory metadata is hidden.
- **Readdir on merged dir**: upper read first, then each lower layer top-to-
  bottom; entries already seen are not re-added. The merged name list is cached
  in the open `struct file` (per-fd). `seekdir(0)` followed by `readdir`
  discards and rebuilds the cache. Changes to the merged dir during an open
  readdir are NOT visible until the cache is rebuilt.
- **Readdir on non-merged dir**: handled directly by the underlying (upper or
  lower) directory.
- **Seek offsets**: assigned sequentially during readdir; not stable across
  close/reopen if the directory changed.

## 5. Whiteouts and Opaque Directories

These mechanisms support `rm`/`rmdir` without modifying the lower filesystem.

### 5.1 Whiteouts
- A whiteout records that a name has been removed in the upper layer.
- **Classic whiteout**: a character device with `rdev == 0/0`.
- **xattr whiteout** (`xwhiteout`): a zero-size regular file with the
  `trusted.overlay.whiteout` (or `user.overlay.whiteout`) xattr. Used for
  nested overlayfs and container-built layers; never created by overlayfs
  itself.
- When a whiteout is found in the upper level of a merged directory, the
  matching lower name is ignored and the whiteout itself is hidden from readdir.

### 5.2 Opaque Directories
- A directory is made opaque by setting `trusted.overlay.opaque` to `"y"`.
- An opaque upper directory hides any lower directory with the same name
  entirely (no merge).
- An opaque directory should not contain whiteouts (they serve no purpose).
- **xwhiteout marker**: a merged directory containing xwhiteout entries should
  set `trusted.overlay.opaque` to `"x"` (not `"y"`) to signal "contains
  xwhiteouts". This avoids the overhead of checking the whiteout xattr on
  every entry during readdir in the common case.

## 6. Non-Directories and Copy Up

- A non-directory object (file, symlink, device, etc.) is presented from the
  upper or lower layer as appropriate.
- **Copy-up trigger**: the first operation requiring write access on a lower
  object — write-open, metadata change (chmod/chown/utime), hardlink, rename,
  xattr set. Creating a symlink does NOT trigger copy-up.
- **Copy-up may be unnecessary**: e.g., opened read-write but data not modified.
- **Copy-up process**:
  1. Ensure the containing directory (and ancestors) exist in the upper fs.
  2. Create the object in the upper fs with the same metadata (owner, mode,
     mtime, symlink target).
  3. If a regular file, copy data from lower to upper.
  4. Copy extended attributes.
- After copy-up, the overlay provides direct access to the new upper file;
  subsequent operations are largely transparent (rename/unlink on the name are
  still handled by the overlay).

## 7. Directory Rename

Renaming a directory that is on the lower layer or merged (not originally
created on the upper layer) is handled in two ways:

1. **Default**: return `EXDEV` ("Invalid cross-device link"). Applications are
   generally prepared for this (e.g., `mv` falls back to recursive copy).
2. **`redirect_dir` feature**: the directory is copied up (metadata only, not
   contents), the `trusted.overlay.redirect` xattr is set to the original path
   from the overlay root, and the directory is moved to the new location.

### `redirect_dir` modes
- `redirect_dir=on`: redirects are created and followed.
- `redirect_dir=follow`: redirects are followed but not created.
- `redirect_dir=nofollow`: redirects are neither created nor followed.
- `redirect_dir=off`: translates to `follow` if
  `CONFIG_OVERLAY_FS_REDIRECT_ALWAYS_FOLLOW`, else `nofollow`.

### Interaction with NFS export
- When `nfs_export=on`, every copied-up directory is indexed by the lower inode
  file handle. On lookup of a merged dir, if the upper dir does not match the
  FH stored in the index, lookup returns an error (possible inconsistency).
- Because lower-layer redirects cannot be verified with the index, enabling
  NFS export on an overlay with **no upper layer** requires
  `redirect_dir=nofollow`.

## 8. Metacopy (Metadata-Only Copy Up)

- When `metacopy=on`, metadata-only operations (chown/chmod) copy up metadata
  only, not data. The upper file is marked with `trusted.overlay.metacopy`
  xattr indicating it contains no data.
- Data is copied up later when the file is opened for WRITE; the `metacopy`
  xattr is then removed.
- **Security warning**: do not use `metacopy=on` with untrusted upper/lower
  directories. An attacker with a handcrafted file (REDIRECT + METACOPY xattrs)
  could gain access to an arbitrary lower file. On local systems setting
  `trusted.*` xattrs requires `CAP_SYS_ADMIN`, but untrusted layers (e.g., USB)
  are vulnerable.
- **Conflicts**: `redirect_dir={off|nofollow|follow[*]}` and `nfs_export=on`
  conflict with `metacopy=on` and result in an error.
  [*] `redirect_dir=follow` only conflicts if `upperdir=` is given.
- **Data-only layers**: specifying at least one data-only layer implicitly
  enables metacopy-style data redirection; other forms of metacopy are then
  rejected. Data-only layers may be used with `userxattr` (careful privilege
  handling required).

## 9. Permission Model

Overlayfs stashes credentials at mount time (the mounter's creds, or with
`override_creds` the calling task's creds via the new mount API). Permission
checking follows three principles:

1. **Consistency**: permission check SHOULD return the same result before and
   after copy-up.
2. **No privilege gain**: the mounting task MUST NOT gain additional privileges
   via the overlay.
3. **Overlay may grant more**: a task MAY gain privileges through the overlay
   compared to direct access on the underlying layers (e.g., server-enforced
   NFS permissions may be ignored).

This is achieved by **two checks on every access**:
- **(a) Local check**: standard DAC (owner/group/mode/POSIX ACL) + MAC on the
  overlay inode using the *current task* credentials. Ensures (1) because
  owner/group/mode/ACLs are copied up.
- **(b) Real check**: check the *stashed* credentials against the real
  underlying upper/lower inode, including MAC. Ensures (2).

`default_permissions` mount option: use only the kernel permission check (skip
the "real" check delegation to underlying fs).

## 10. Mount Options (Authoritative List)

| Option                   | Values                 | Semantics                                                                           |
| :----------------------- | :--------------------- | :---------------------------------------------------------------------------------- |
| `lowerdir` / `lowerdir+` | path(s)                | Lower layers. Old API: colon-separated. New API: `lowerdir+` appends.               |
| `upperdir`               | path                   | Writable upper layer. Omit for read-only overlay.                                   |
| `workdir` / `workdir+`   | path                   | Work directory, must be on the same fs as `upperdir`, must be empty.                |
| `datadir+`               | path                   | Data-only lower layers (metacopy).                                                  |
| `default_permissions`    | bool                   | Kernel-only permission check.                                                       |
| `redirect_dir`           | on/follow/nofollow/off | Directory redirect feature (see §7).                                                |
| `index`                  | bool                   | Hardlink preservation via index directory.                                          |
| `uuid`                   | null/off/on/auto       | Overlay instance UUID / `fsid` source (see §12).                                    |
| `nfs_export`             | bool                   | NFS export support (requires `index=on` for read-write).                            |
| `xino`                   | off/auto/on            | Unified `st_ino`/`st_dev` via xino bits (see §3).                                   |
| `metacopy`               | bool                   | Metadata-only copy up (see §8).                                                     |
| `userxattr`              | bool                   | Use `user.overlay.*` namespace instead of `trusted.overlay.*` (unprivileged mount). |
| `fsync`                  | volatile/auto/strict   | Durability during copy-up (see §13).                                                |
| `verity`                 | off/on/require         | fs-verity metacopy digest (see §14).                                                |
| `override_creds`         | bool                   | Use caller task creds as creator creds (new mount API only, v6.15+).                |

### Layer specification via file descriptors (v6.13+)
`datadir+`, `lowerdir+`, `upperdir`, `workdir+` accept file descriptors via
`fsconfig(FSCONFIG_SET_FD)` in the new mount API.

### Colon escaping
- Old API: colons in lowerdir names are escaped with a single backslash.
- New API (v6.8+): `lowerdir+` / `datadir+` accept raw names; colons are
  escaped as octal `\072` in `/proc/self/mountinfo`.

## 11. Nesting and Xattr Escaping

- A lower directory may itself be an overlayfs mount.
- Overlay-specific xattrs (`overlay.*`) would be interpreted and stripped by
  the underlying overlayfs. To preserve them through nesting, they are
  **escaped** with the prefix `overlay.overlay.`. Each un-nesting removes one
  prefix; nesting repeats the prefix.
- A regular whiteout in a lower overlay dir is always handled by that lower
  overlay. To store an *effective* whiteout in an overlay lower dir, use the
  xattr whiteout form (zero-size file with `overlay.whiteout` xattr inside a
  dir marked `overlay.opaque=x`). These alternative whiteouts are never created
  by overlayfs itself; they are created by userspace (e.g., container tools).
- Alternative whiteouts can be escaped via the standard xattr escape mechanism
  for arbitrary nesting depth.

## 12. UUID and `fsid`

- `uuid=null`: overlay UUID is null; `fsid` taken from the topmost underlying fs.
- `uuid=off`: same as `null`, plus underlying-layer UUIDs are ignored (null
  used in file handles). Useful when the underlying disk is copied and its UUID
  changes. Only applicable if all lower dirs are on the same fs.
- `uuid=on`: overlay UUID is generated and stored in `trusted.overlay.uuid`
  xattr; `fsid` is unique and persistent. Requires an upper fs that supports
  xattrs.
- `uuid=auto` (default): take UUID from `trusted.overlay.uuid` if it exists;
  upgrade to `on` on first mount of a new overlay meeting prerequisites;
  downgrade to `null` for existing overlays never mounted with `uuid=on`.

## 13. Durability and Copy Up

- `fsync(2)` guarantees data and metadata are safely on backing storage.
- Without `fsync`, no guarantee about post-crash observed data (old, new, or a
  mix; possibly zeros if a copy-up was interrupted mid-flight).
- Overlayfs calls `fsync` on the upper file **before** completing data copy-up
  via `rename`/`link` to make the copy-up atomic. This prevents the upper file
  from ending up as zeros after a crash.
- **`fsync=auto` (default)**: `fsync` upper file before data copy-up
  completion. No explicit `fsync` on directory or metadata-only copy-up.
- **`fsync=strict`**: `fsync` upper file AND directories before completion of
  any copy-up.
- **`fsync=volatile`** (alias `volatile`): omit all sync calls to the upper fs.
  See §15.
- On traditional local journaling filesystems (ext4, xfs), `fsync` on a file
  also persists parent directory changes (same transaction), so metadata
  durability during data copy-up is effectively free.
- Network filesystems are **disallowed** as upper layer (further risk limiting).

## 14. fs-verity Support

- During metadata copy-up of a lower file with fs-verity enabled, if overlay
  verity is enabled, the digest of the lower file is stored in the
  `trusted.overlay.metacopy` xattr.
- The digest is used to verify the lower file content each time the metacopy
  file is opened.
- If the lower file is replaced/modified after mount, access to the overlay
  file returns `EIO` (on open via digest check, or on later read via fs-verity)
  and an error is logged.
- **`verity=off`** (default): digest never generated or used.
- **`verity=on`**: if a metacopy file specifies a digest, the data file must
  match it. When generating a metacopy, the verity digest is set from the
  source (if it has one).
- **`verity=require`**: same as `on`, plus all metacopy files MUST specify a
  digest (`EIO` on open otherwise). Metadata copy-up is only used when the data
  file has fs-verity enabled; otherwise a full copy-up is used.
- **Trust use case**: if the upper layer is fully trusted (e.g., dm-verity), an
  untrusted lower layer can supply validated content for all metacopy files.
  Combined with data-only untrusted lowers, the entire mount can be trusted to
  match the upper layer.

## 15. Volatile Mount

- `volatile` (alias `fsync=volatile`): all sync calls to the upper fs are
  omitted. Not guaranteed to survive a crash. Recommended only when overlay
  data can be recreated without significant effort.
- **Syncfs/fsync semantics**: if any writeback error occurs on the upperdir fs
  after a volatile mount, all sync functions return an error for the rest of
  the mount's lifetime (no recovery).
- **Crash indicator**: the directory `$workdir/work/incompat/volatile` is
  created on volatile mount. On the next mount, overlayfs refuses to mount if
  this directory is present — a strong indicator that the user should discard
  upper and work directories and create fresh ones. In limited known-good
  cases, the user may manually remove the `volatile` directory.

## 16. Sharing and Copying Layers

- Lower layers may be shared among several overlay mounts and may overlap
  (beneath/above another overlay's lower path).
- Using an `upperdir`/`workdir` already used by another overlay mount is NOT
  allowed and may fail with `EBUSY`. Partially overlapping paths are also not
  allowed (`EBUSY`). Behavior of two overlays sharing/overlapping upper/workdir
  is undefined (no crash/deadlock).
- Reusing an upper path with a *different* lower path is allowed UNLESS
  `index` or `metacopy` is enabled.
- **`index` feature**: on first mount, an NFS file handle of the lower root
  dir + the lower fs UUID are encoded into `trusted.overlay.origin` on the
  upper root. On subsequent mounts, the lower root FH and UUID are compared to
  the stored origin; mismatch fails the mount with `ESTALE`. Requires the
  lower fs to support NFS export, have a valid UUID, and the upper fs to support
  xattrs; otherwise `EOPNOTSUPP`.
- **`metacopy` feature**: no mount-time verification. Mounting the same upper
  with different lowers may succeed but behavior is undefined. Do not do it.
- Copying layers to a different dir/machine: with `index`, the copied layers
  fail the lower-root FH verification.

## 17. NFS Export

- When underlying filesystems support NFS export and `nfs_export=on`, the
  overlay may be exported to NFS.
- **Index entry**: on copy-up of any lower object, an index entry is created
  under the index directory. The entry name is the hex copy-up origin FH.
  - Non-directory: index entry is a hard link to the upper inode.
  - Directory: index entry has `trusted.overlay.upper` xattr with an encoded FH
    of the upper directory inode.
- **File-handle encoding rules**:
  1. Non-upper object → encode a lower FH from the lower inode.
  2. Indexed object → encode a lower FH from the copy-up origin.
  3. Pure-upper or non-indexed upper → encode an upper FH from the upper inode.
- Encoded overlay FH includes: header with path type (lower/upper), UUID of the
  underlying fs, and the underlying fs's encoding of the underlying inode.
  Format is identical to the `trusted.overlay.origin` xattr format.
- **Decoding steps**:
  1. Find underlying layer by UUID and path type.
  2. Decode the underlying fs FH to an underlying dentry.
  3. For a lower FH, look it up in the index directory by name.
  4. If a whiteout is found in the index, return `ESTALE` (object was deleted
     after FH encoding).
  5. Non-directory: instantiate a disconnected overlay dentry.
  6. Directory: use the connected underlying decoded dentry to look up a
     connected overlay dentry.
- **Middle-layer redirects**: a middle-layer directory may have a `redirect` to
  a lower dir. Middle-layer redirects are NOT indexed, so a lower FH encoded
  from a redirect origin (or its descendant) cannot reconstruct a connected
  overlay path. Mitigation: such directories are copied up on encode and
  encoded as an upper FH. On an overlay with no upper layer this mitigation is
  unavailable → NFS export requires `redirect_dir=nofollow`.
- **Limitations**:
  - Overlay does not support non-directory connectable file handles.
    `subtree_check` exportfs config causes NFS lookup failures.
  - When `nfs_export=on`, all directory index entries are verified at mount
    time (potential significant overhead).
  - `index=off` + `nfs_export=on` conflict for read-write mounts → error.
  - `uuid=off` can relax UUID checks (useful after disk copy with UUID change);
    only applicable if all lower dirs are on the same fs.

## 18. Changes to Underlying Filesystems

- **Online changes** (while overlay is mounted): NOT allowed. Behavior is
  undefined (no crash/deadlock).
- **Offline changes** (overlay not mounted):
  - Upper tree: allowed.
  - Lower tree: allowed ONLY if `metacopy`, `index`, `xino`, and `redirect_dir`
    have NOT been used. If any of these features was used and the lower tree is
    modified, behavior is undefined (no crash/deadlock).
- When `nfs_export=on`, behavior on offline lower changes differs: every
  copy-up stores the lower inode FH + lower fs UUID in `trusted.overlay.origin`
  on the upper inode. On lookup of a merged dir, if the found lower dir FH/UUID
  does not match the stored origin, the directory is NOT merged with the upper.

## 19. Non-Standard Behavior (POSIX Divergences)

Overlayfs is "mostly POSIX compliant." Known divergences:

a. **`st_atime` on lower-layer reads**: POSIX mandates updating `st_atime` on
   reads. Overlayfs does NOT do this for files residing on a lower layer.
b. **`MAP_SHARED` of lower-layer files**: if a lower file is opened read-only
   and memory-mapped `MAP_SHARED`, subsequent changes to the file are NOT
   reflected in the mapping.
c. **`ETXTBSY` on executing lower files**: if a lower file is being executed,
   opening it for write or truncating it is NOT denied with `ETXTBSY`.

The `redirect_dir`, `index`, and `xino` options (§7, §16, §3) bring overlayfs
closer to standards-compliant behavior.

## 20. Xattr Namespaces

- **`trusted.overlay.*`** — default namespace; requires `CAP_SYS_ADMIN` to set.
- **`user.overlay.*`** — enabled by `userxattr` mount option; allows
  unprivileged mounting.
- **Escape prefix** (`overlay.overlay.`) — for nesting (§11).
- **Private xattrs** (`overlay.*`) are filtered from `listxattr` output; they
  are not directly visible to userspace.

## 21. Testsuite

The upstream testsuite is `unionmount-testsuite` (David Howells / Amir
Goldstein): https://github.com/amir73il/unionmount-testsuite.git

Run as root: `./run --ov --verify`

For the Asterinas refactor, the upstream-approved validation lane is xfstests
(see `XFSTESTS_PREBUILT_IMAGE_GUIDE.md`).

## 22. Scope Guidance for the Asterinas Refactor

The spec defines a large feature surface. The Architect MUST decide which
features are in scope for the initial refactor wave. A **minimal viable
overlayfs** can omit:
- `index`, `nfs_export` (hardlink preservation / NFS export)
- `metacopy`, `verity` (metadata-only copy-up / fs-verity)
- `redirect_dir` (return `EXDEV` on cross-dir rename instead)
- `xino` (report overlay-internal dev/ino uniformly; less POSIX-compliant but
  simpler)
- `userxattr` (start with `trusted.*` namespace)
- `fsync=volatile`/`strict` (start with `auto` semantics)
- Data-only lower layers
- Layer specification via FDs (new mount API feature)

The **mandatory** core for a useful overlayfs is:
- Upper + lower layer stacking (§2)
- Merged directories with readdir dedup (§4)
- Whiteouts and opaque directories (§5)
- Non-directory copy-up (full data copy-up, §6)
- Two-step permission model (§9)
- `EXDEV` on cross-dir rename (§7 default)
- `st_dev`/`st_ino` reporting (some consistent choice, §3)

The Architect records the scope decision in the Global Topology artifact; the
Designer's validation contract MUST only require upstream-approved validation
for the in-scope feature set.
