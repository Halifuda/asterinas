<!-- SPDX-License-Identifier: MPL-2.0 -->

# Plan: Rebuild the Existing PR Branch as 8 Readable Commits (2026-08-19)

**Status:** RECORDED ONLY — not executed.

## Goal

- Make `~/asterinas-pr` branch `codex/pr-overlayfs-refactor`'s production overlayfs code exactly match the dev branch's active production overlayfs code.
- Keep the PR as the same branch (no new branch).
- Keep exactly 8 commits.
- Make each commit reviewable as a coherent module slice.
- Update the already-open PR via force-push when executed.

## Scope Definition

- "Production code" = active overlayfs implementation under `kernel/core/src/fs/fs_impls/overlayfs/`.
- Excluded from the equality check:
  - `.agents/`
  - `legacy_fs.rs` (dead in dev; already removed in PR)
- Also include PR-relevant non-overlayfs files if they differ:
  - VFS changes under `kernel/core/src/fs/vfs/`
  - utils changes under `kernel/core/src/fs/utils/`
  - `test/initramfs/src/regression/fs/overlayfs/readdir_small_buffer.c`

## Current Facts (read-only research)

- PR branch currently has 8 commits on `upstream/main` = `76dac6f55`.
- PR production overlayfs tree is effectively equal to dev pre-wave-9 commit `4084b5992`, except:
  - PR already removed `legacy_fs.rs`;
  - dev still keeps `legacy_fs.rs` as an inactive file.
- The missing delta is the combined wave-9 diff:
  ```text
  4084b5992..0692ec06b
  ```
  restricted to active overlayfs production files.
- The two wave-9 commits are:
  - `978754581 wave-9: comments audit`
  - `0692ec06b wave-9: code structure`

## Why Not fixup+autosquash

- fixup+autosquash is feasible but requires manually splitting the wave-9 delta into many fixup commits.
- The wave-9 delta contains file moves and cross-cutting `mod.rs` changes.
- Applying those as fixups to the old 8 commits would make intermediate commits noisy and harder to review.

## Chosen Approach

Rewrite the existing `codex/pr-overlayfs-refactor` branch in place:

1. Backup the current branch locally (optional but recommended):
   ```bash
   git branch codex/pr-overlayfs-refactor-backup-20260819
   ```

2. Reset the existing PR branch to `upstream/main`:
   ```bash
   git checkout codex/pr-overlayfs-refactor
   git reset --hard upstream/main
   ```

3. Take the final active overlayfs production files from dev HEAD `0692ec06b`:
   ```bash
   DEV=/home/ayd/asterinas
   git -C "$DEV" ls-tree -r --name-only 0692ec06b -- \
     kernel/core/src/fs/fs_impls/overlayfs \
     | grep -v '/.agents/' \
     | grep -v 'legacy_fs.rs'
   ```

4. Build 8 commits using the original logical boundaries, but with final file locations:

   | # | Commit | Files |
   |---|---|---|
   | C1 | mount resource and layer-stack assembly | `mount/*`, root `superblock.rs`, VFS changes |
   | C2 | visibility projection and identity resolution | `projection/{mod,identity,lower_id,inode_cache,lookup}.rs`, dentry changes |
   | C3 | inode projection and merged-directory readdir index | `projection/binding_cache.rs`, root `inode.rs`, `readdir_index.rs` |
   | C4 | copy-up and file-view promotion | `copyup/*`, root `workdir.rs` |
   | C5 | directory mutation and whiteouts | `dir/*` |
   | C6 | metadata, permission, and xattr policy | `metadata_security/*` |
   | C7 | register new overlayfs and remove legacy | final `mod.rs`, `fs_type.rs`, `utils/*`, delete legacy |
   | C8 | align readdir_small_buffer regression test | `test/.../readdir_small_buffer.c` |

5. Per-commit construction pattern:
   ```bash
   git checkout 0692ec06b -- <files for this commit>
   git commit -m "<commit subject>"
   ```

6. C1–C6 use story-first:
   - Add files only.
   - Do not wire `mod.rs` until C7.
   - Intermediate commits may not compile.
   - C7 writes the final `mod.rs`, declares all modules, registers the new fs, and removes legacy.

7. Final equality check:
   ```bash
   git diff 0692ec06b HEAD -- \
     kernel/core/src/fs/fs_impls/overlayfs \
     ':(exclude)kernel/core/src/fs/fs_impls/overlayfs/.agents' \
     ':(exclude)kernel/core/src/fs/fs_impls/overlayfs/legacy_fs.rs'
   ```
   Expected: no output.

   Also check:
   ```bash
   git diff 0692ec06b HEAD -- \
     kernel/core/src/fs/vfs \
     kernel/core/src/fs/utils \
     test/initramfs/src/regression/fs/overlayfs/readdir_small_buffer.c
   ```

8. Validation after C7:
   ```bash
   cargo check -p asterinas --target x86_64-unknown-none
   make check
   make docs
   ```

9. Update the existing PR with force-push:
   ```bash
   git push --force-with-lease origin codex/pr-overlayfs-refactor
   ```

## Risks / Notes

- This rewrites PR history; commit hashes will change.
- Force-push is required to update the existing PR.
- Keep a local backup branch before rewriting.
- If exact inclusion of `legacy_fs.rs` is required, the scope definition must be revisited.
