<!-- SPDX-License-Identifier: MPL-2.0 -->

# Filesystem Spec Index — Overlayfs

Quick-lookup index into the authoritative overlayfs specification. Use this to
jump to the right rule when mapping a micro-feature or reviewing a design
contract.

## Column Key

- **Spec §**: section number in `FILESYSTEM_SPEC_SUMMARY.md`
- **rst L**: line range in `~/linux/Documentation/filesystems/overlayfs.rst`
- **Source**: Linux source file(s) implementing the rule (for cross-reference
  with `REFERENCE_IMPLEMENTATION_SUMMARY.md`)

## 1. Topical Index

| Topic                                        | Spec § |      rst L      | Source                   | Rule (one-line)                                                         |
| :------------------------------------------- | :----: | :-------------: | :----------------------- | :---------------------------------------------------------------------- |
| Definition / hybrid stacking                 |   1    |      6-15       | `super.c`                | Overlay unifies upper + lower dir trees; owns no on-disk format.        |
| Layer model / ordering                       |   2    | 83-108, 350-378 | `super.c`, `ovl_entry.h` | Upper optional; lowers stacked top→bottom; data-only layers below `::`. |
| Inode identity (`st_dev`/`st_ino`/`d_ino`)   |   3    |      32-82      | `inode.c`, `readdir.c`   | xino table; same-fs vs xino vs overflow behavior.                       |
| Merged directory formation                   |   4    |     110-167     | `namei.c`, `readdir.c`   | Both dirs → merge; non-dir hides lower; metadata from upper only.       |
| Readdir on merged dir                        |   4    |     169-198     | `readdir.c`              | Upper first, then lowers; per-fd cache; `seekdir(0)` rebuilds.          |
| Whiteouts (classic + xattr)                  |  5.1   |     141-166     | `dir.c`, `util.c`        | Char dev 0/0 OR zero-size file with `overlay.whiteout` xattr.           |
| Opaque directories                           |  5.2   |     141-166     | `dir.c`, `xattrs.c`      | `overlay.opaque=y` hides lower dir; `=x` signals xwhiteouts present.    |
| Non-directory copy-up trigger                |   6    |     199-264     | `copy_up.c`, `file.c`    | Write-open/metadata/hardlink/rename/xattr → copy up.                    |
| Copy-up process                              |   6    |     199-264     | `copy_up.c`              | Parent first, then metadata, then data, then xattrs.                    |
| Directory rename (EXDEV default)             |   7    |     200-263     | `dir.c`                  | Lower/merged dir rename returns `EXDEV` by default.                     |
| `redirect_dir` modes                         |   7    |     200-263     | `dir.c`, `params.c`      | on/follow/nofollow/off; copies up dir + sets redirect xattr.            |
| Metacopy (metadata-only copy-up)             |   8    |     379-413     | `copy_up.c`              | `overlay.metacopy` xattr; data deferred to write-open.                  |
| Metacopy security warning                    |   8    |     379-413     | —                        | Do not use with untrusted layers (REDIRECT+METACOPY attack).            |
| Metacopy option conflicts                    |   8    |     379-413     | `params.c`               | `redirect_dir`/`nfs_export` conflict with `metacopy=on`.                |
| Data-only lower layers                       |   8    |     414-463     | `super.c`                | `::` separator; implicit metacopy; no names/metadata visible.           |
| Permission model (two-step)                  |   9    |     292-349     | `inode.c`, `util.c`      | Local DAC+MAC (current creds) + real check (stashed creds).             |
| `default_permissions`                        | 9, 10  |     292-349     | `params.c`, `inode.c`    | Kernel-only permission check.                                           |
| Mount options (full table)                   |   10   |        —        | `params.c`               | See Spec §10 for authoritative list + values.                           |
| Layer spec via FDs (v6.13+)                  |   10   |     464-481     | `params.c`               | `FSCONFIG_SET_FD` for lowerdir+/datadir+/upperdir/workdir+.             |
| Colon escaping                               |   10   |     350-378     | `params.c`               | Old API: `\:`; new API: `lowerdir+` raw, `\072` in mountinfo.           |
| Nesting / xattr escaping                     |   11   |     568-593     | `xattrs.c`               | `overlay.overlay.` prefix; one prefix removed per un-nest.              |
| Alternative (xattr) whiteouts                |   11   |     568-593     | `dir.c`, `xattrs.c`      | Zero-size file + `overlay.whiteout` in `overlay.opaque=x` dir.          |
| UUID / `fsid` modes                          |   12   |     762-785     | `super.c`, `util.c`      | null/off/on/auto; `overlay.uuid` xattr; `fsid` source.                  |
| Durability / `fsync` during copy-up          |   13   |     786-835     | `copy_up.c`, `file.c`    | `fsync` upper temp before atomic rename; auto/strict/volatile.          |
| fs-verity support                            |   14   |     482-527     | `copy_up.c`              | Digest in `overlay.metacopy`; off/on/require.                           |
| Volatile mount                               |   15   |     836-863     | `super.c`, `file.c`      | Omit all sync; `incompat/volatile` dir as crash marker.                 |
| Layer sharing / `EBUSY`                      |   16   |     528-567     | `super.c`                | Lowers shareable; upper/workdir exclusive; `EBUSY` on overlap.          |
| `index` feature (origin verification)        |   16   |     528-567     | `namei.c`, `copy_up.c`   | `overlay.origin` on upper root; `ESTALE` on mismatch.                   |
| NFS export                                   |   17   |     684-761     | `export.c`, `namei.c`    | Index dir + FH encode/decode; `ESTALE` on whiteout-in-index.            |
| Middle-layer redirect mitigation             |   17   |     684-761     | `export.c`               | Copy up on encode; no-upper requires `redirect_dir=nofollow`.           |
| Online/offline underlying changes            |   18   |     651-683     | —                        | Online changes forbidden; offline lower changes restricted.             |
| POSIX divergences (atime/MAP_SHARED/ETXTBSY) |   19   |     594-650     | `file.c`, `inode.c`      | Three known non-POSIX behaviors on lower-layer files.                   |
| Xattr namespaces                             |   20   |     864-871     | `xattrs.c`               | `trusted.overlay.*` (default) / `user.overlay.*` (`userxattr`).         |
| Testsuite                                    |   21   |     872-883     | —                        | `unionmount-testsuite`; xfstests for Asterinas lane.                    |
| Scope guidance (minimal viable set)          |   22   |        —        | —                        | Mandatory core vs optional advanced features for the refactor.          |

## 2. Xattr Quick Reference

| Xattr                      | Spec §  | Constant              | Set By                   | Read By                          |
| :------------------------- | :-----: | :-------------------- | :----------------------- | :------------------------------- |
| `trusted.overlay.opaque`   |   5.2   | `OVL_XATTR_OPAQUE`    | `ovl_set_opaque`         | `ovl_get_opaquedir_val`          |
| `trusted.overlay.whiteout` | 5.1, 11 | `OVL_XATTR_XWHITEOUT` | userspace (xwhiteout)    | `ovl_path_check_xwhiteout_xattr` |
| `trusted.overlay.redirect` |    7    | `OVL_XATTR_REDIRECT`  | `ovl_set_redirect`       | `ovl_get_redirect_xattr`         |
| `trusted.overlay.origin`   | 16, 17  | `OVL_XATTR_ORIGIN`    | `ovl_set_origin_fh`      | `ovl_check_origin_fh`            |
| `trusted.overlay.impure`   |    —    | `OVL_XATTR_IMPURE`    | `ovl_set_impure`         | `ovl_is_impuredir`               |
| `trusted.overlay.nlink`    |    —    | `OVL_XATTR_NLINK`     | copy-up (hardlink)       | `ovl_get_nlink`                  |
| `trusted.overlay.upper`    |   17    | `OVL_XATTR_UPPER`     | `ovl_create_index`       | `ovl_index_upper`                |
| `trusted.overlay.uuid`     |   12    | `OVL_XATTR_UUID`      | `ovl_init_uuid_xattr`    | uuid mount verification          |
| `trusted.overlay.metacopy` |  8, 14  | `OVL_XATTR_METACOPY`  | `ovl_set_metacopy_xattr` | `ovl_check_metacopy_xattr`       |
| `trusted.overlay.protattr` |    —    | `OVL_XATTR_PROTATTR`  | `ovl_set_protattr`       | `ovl_check_protattr`             |

## 3. Mount Option Quick Reference

| Option                   | Spec § |      rst L      | Values                 | Default                  |
| :----------------------- | :----: | :-------------: | :--------------------- | :----------------------- |
| `lowerdir` / `lowerdir+` | 2, 10  | 83-108, 350-378 | path(s)                | —                        |
| `upperdir`               | 2, 10  |     83-108      | path                   | — (ro if omitted)        |
| `workdir` / `workdir+`   | 2, 10  |     83-108      | path                   | —                        |
| `datadir+`               | 8, 10  |     414-463     | path                   | —                        |
| `default_permissions`    | 9, 10  |     292-349     | bool                   | off                      |
| `redirect_dir`           | 7, 10  |     200-263     | on/follow/nofollow/off | off (Kconfig-dependent)  |
| `index`                  | 16, 10 |     528-567     | bool                   | off (Kconfig-dependent)  |
| `uuid`                   | 12, 10 |     762-785     | null/off/on/auto       | auto                     |
| `nfs_export`             | 17, 10 |     684-761     | bool                   | off                      |
| `xino`                   | 3, 10  |      32-82      | off/auto/on            | off (auto if Kconfig)    |
| `metacopy`               | 8, 10  |     379-413     | bool                   | off (Kconfig-dependent)  |
| `userxattr`              | 20, 10 |     864-871     | bool                   | off                      |
| `fsync`                  | 13, 10 |     786-835     | volatile/auto/strict   | auto                     |
| `verity`                 | 14, 10 |     482-527     | off/on/require         | off                      |
| `override_creds`         | 9, 10  |     292-349     | bool                   | off (new mount API only) |

## 4. Error Code Quick Reference

| Errno          | Spec § | When                                                              |
| :------------- | :----: | :---------------------------------------------------------------- |
| `EXDEV`        |   7    | Cross-dir rename without `redirect_dir`.                          |
| `ESTALE`       | 16, 17 | `index` origin mismatch on mount; whiteout in index on FH decode. |
| `EBUSY`        |   16   | Upper/workdir already used or overlapping.                        |
| `EOPNOTSUPP`   |   16   | `index=on` but lower fs lacks NFS export / UUID / xattr support.  |
| `EIO`          | 14, 17 | fs-verity digest mismatch; NFS export stale FH.                   |
| `EROFS`        |   2    | Read-only overlay (no upperdir) rejects mutative ops.             |
| `EINVAL`       |   10   | Invalid mount option value.                                       |
| `ENAMETOOLONG` |   —    | Name exceeds layer namelen.                                       |

## 5. Source File Quick Reference

| File          | Spec § (primary) | Lines | Role                                                               |
| :------------ | :--------------: | ----: | :----------------------------------------------------------------- |
| `ovl_entry.h` |       2, 3       |   193 | Core structures (`ovl_fs`, `ovl_inode`, `ovl_entry`, `ovl_layer`). |
| `overlayfs.h` |      5, 20       |   954 | Enums, accessors, `ovl_do_*` VFS wrappers.                         |
| `params.h`    |        10        |    44 | `ovl_apply_options` prototype.                                     |
| `super.c`     |   1, 2, 12, 15   |  1622 | Mount, superblock, layer setup, inode cache.                       |
| `params.c`    |        10        |  1104 | Mount option parsing.                                              |
| `namei.c`     |   4, 7, 16, 17   |  1483 | Lookup, redirect, origin/index verification.                       |
| `dir.c`       |       5, 7       |  1496 | Dir mutations, whiteout, opaque, redirect.                         |
| `readdir.c`   |       4, 5       |  1326 | Merged readdir cache, whiteout enumeration.                        |
| `inode.c`     |       3, 9       |  1298 | Attributes, permission, ACL, fileattr.                             |
| `file.c`      |    6, 13, 19     |   652 | File ops, copy-up-on-open, fsync.                                  |
| `copy_up.c`   |   6, 8, 14, 16   |  1291 | Full/metacopy copy-up, origin FH, index.                           |
| `export.c`    |        17        |   868 | NFS export FH encode/decode.                                       |
| `util.c`      |        9         |  1518 | Path-type, accessors, creds, inuse lock.                           |
| `xattrs.c`    |      11, 20      |   261 | Xattr handlers, escape, private classification.                    |
