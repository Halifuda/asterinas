<!-- SPDX-License-Identifier: MPL-2.0 -->

# 2026-08-10 Upstream Rebase + VFS API Repair — Change Record

Branch: `codex/overlayfs-refactor` · Commits: rebase (29 commits preserved)
+ `e16075a72` (adaptation). Companion handoff:
`main-agent/20260810-rebase-upstream-api-repair_main_agent_handoff.md`.

## 1. Rebase summary

- `upstream/main` advanced `e0e79a954..94a8f624d` (72 commits); branch
  rebased onto `94a8f624d`, all 29 local commits preserved (no squash).
- Only conflict: commit 7/29 (`refactor exfat implementation`) — resolved in
  favor of the local refactored exFAT per user decision: `bitmap.rs`,
  `fs.rs`, `mod.rs` kept ours; `inode.rs` / `upcase_table.rs` deletions kept
  (restored later by our `Reorganize the sub-modules under the fs module`
  commit). Final exFAT tree byte-identical to pre-rebase tip.
- `overlayfs/fs.rs` (renamed by us to `legacy_fs.rs`) was 3-way merged with
  upstream's new-VFS-API adaptation of that file.
- Safety backup tag: `backup/ovfs-pre-upstream-rebase-20260810` ->
  `cf8547536` (old tip).

## 2. Upstream API adaptations (commit e16075a72, 19 files, +157/-60)

| Area | Change |
|---|---|
| `Inode::metadata()` | Now returns `Result<Metadata>`. Trait impls updated in `exfat/inode/mod.rs` and `overlayfs/projection/inode.rs`; all callers propagate `?` (`metadata_security/metadata.rs`, `permission.rs`, `copyup/promote.rs`, `dir/mod.rs`, `mount/claims.rs`, `mount/layers.rs`, `projection/entry.rs`, `copyup/mod.rs`). Infallible `best_effort_time_set` uses `.ok()` with early return. |
| `Inode::page_cache()` | Now returns `Option<Arc<Vmo>>` (was `Option<PageCache>`). Impls updated in `exfat/inode/mod.rs` and `overlayfs/projection/inode.rs`/`copyup/mod.rs`. |
| `PageCache` | No longer `Clone`; `resize` now takes `&mut self`. exFAT stores `Once<Option<Mutex<PageCache>>>` (`exfat/inode/mod.rs`, `page_backend.rs`); all access sites lock (`file_mutation.rs`, `dir_mutation/growth.rs`, `parent_entry_set.rs`, `sync.rs`, `file_read.rs`, `page_backend.rs`, `inode/mod.rs`). |
| `FsType` | New required `type Key` + `create(&mut FsCreationCtx)`: `exfat/fs.rs`, `overlayfs/mount/mod.rs`. |
| `Vmo::commit_on` | Now takes `(page_idx, VmoMapMode)` and returns `Result<()>`; removed frame `writer()`. `exfat/inode/file_mutation/page_cache_growth.rs` uses `commit_on(idx, SharedWrite)` + `PageCache::fill_zeros`. |
| `exfat/fs.rs::create` | Parses options (immutable) before cloning `resolve_block_device()` result to break the `&mut` borrow. |
| Clippy docs | 4 new `doc_lazy_continuation` errors in `overlayfs/dir/mod.rs` (upstream's newer clippy) fixed by indenting continuation lines. |

## 3. Validation evidence

- `docker exec -w /root/asterinas codex-asterinas-dev bash -lc 'cd
  /root/asterinas/kernel && cargo check -p aster-kernel --target
  x86_64-unknown-none'` → **exit 0**; only pre-existing warning
  `MountPolicy::uuid_mode` dead field (`mount/policy.rs`, documented by
  prior handoffs, intentionally untouched).
- `./tools/clippy_check.sh workspace` → fails only on the same pre-existing
  `uuid_mode` dead-code error.
- rustfmt: 4 edited exFAT files `--check` clean. 24 pre-existing formatting
  diffs elsewhere in `overlayfs/` remain under the container's newer rustfmt
  (untouched; separate toolchain-formatting concern).
- Recommended before next pass: full `docker exec -w /root/asterinas
  codex-asterinas-dev make kernel`.

## 4. Out of scope / left untouched

- `MountPolicy::uuid_mode` dead field (workspace precedent: intentionally
  left).
- 24 pre-existing rustfmt diffs under the newer toolchain.
- No push performed; `codex/overlayfs-refactor` diverges from
  `origin/codex/overlayfs-refactor` (force-push only on explicit request).
