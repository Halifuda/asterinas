<!-- SPDX-License-Identifier: MPL-2.0 -->

# EXR-INODE-05B Advisor Serial

## Blocker Summary

The serial batch is blocked by a compile-time compatibility defect in `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`.

- `FatAttr` uses a `bitflags!` declaration that conflicts with the derived `Clone`, `Copy`, `Debug`, `Eq`, and `PartialEq` implementations under the workspace's macro expansion.
- `FatAttr::from_bits_retain` is not available in this repo's `bitflags` surface, so the current constructor call does not compile.

This is a narrow integration/style repair, not a metadata redesign. The creator should align the refactor inode's `FatAttr` usage with the repo-compatible pattern already used by legacy exfat code, or with an equivalent local `bitflags` form that the workspace supports.

## Repair Boundaries

The creator may touch only:

- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`

No other production files are in scope for this batch.

## Required Change

1. Make `FatAttr` compile with the workspace's `bitflags` macro usage.
   - Remove the incompatible derive/macro combination or rewrite the declaration into the repo-compatible form already demonstrated in `kernel/src/fs/fs_impls/exfat/inode.rs`.
   - Keep `FatAttr` as the local exFAT attribute carrier for this inode shell.

2. Replace the unavailable `from_bits_retain` call with the supported construction pattern in this workspace.
   - Use the local `bitflags` API variant that compiles here.
   - Do not introduce new semantic rules for file attributes.

3. Preserve the existing metadata-shell behavior exactly where it already compiles.
   - Ordinary inode construction still accepts validated file-record facts and validated chain facts.
   - Root inode construction still goes through `new_root(...)` only.
   - Directory records still reject mismatched valid-data length and data length.
   - Accessors still return stored values only.

## Must Remain Unchanged

- The repair must not widen into page-cache ownership, buffered I/O, mount sequencing, VFS inode behavior, registry mutation, or directory iteration.
- The ordinary constructor must still reject the reserved root key.
- The root constructor must still require the reserved root key and keep the root shell synthetic.
- The file-versus-directory distinction must still be preserved from the validated file record.
- Raw UTF-16 name units must still be preserved exactly as exposed by the validated file-record boundary.
- The checker-added ktests in this file must continue to express the same behavior targets after the compile fix.

## Done Criteria

The batch is done when:

- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` compiles cleanly with the workspace's `bitflags` support.
- The serial ktest run can proceed past the `FatAttr` build failure.
- The existing inode metadata-shell tests still validate the same behavior:
  - preserved validated file-record facts,
  - explicit synthetic root construction,
  - directory length mismatch rejection,
  - pure read-only accessors.

If the creator finds that preserving unknown attribute bits requires a different supported constructor than the legacy pattern, keep the change local and document that as an integration compatibility choice, not as a semantics change.
