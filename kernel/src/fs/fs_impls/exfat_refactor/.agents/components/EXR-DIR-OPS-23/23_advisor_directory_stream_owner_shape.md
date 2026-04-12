<!-- SPDX-License-Identifier: MPL-2.0 -->

# Advisor Result

## Metadata

- Component ID: `EXR-DIR-OPS-23`
- Role: `advisor`
- Phase: `owner-shape analysis`
- Date: `2026-04-12`
- Packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-OPS-23/20260412-0853-advisor-directory-stream-owner-shape-packet.md`

## Diagnosis

- The current `directory_stream` bridge should remain `ExfatFs`-owned, not move into a second inode-owned stream owner.
- `ExfatInode` should stay the directory-method surface and may only own a thin private wrapper that packages its validated chain facts into the filesystem-owned bridge.
- The concurrency concern is mostly code-shape friction, not a new shared-state hazard: `ExfatInode::readdir_at()` and `lookup()` already upgrade the weak filesystem back-reference per call, then immediately create a fresh stream and drop back out of the scan path; they do not retain a shared scanner across calls.

## Why This Fits The Protocol

- The designer spec explicitly says `ExfatInode::lookup` and `ExfatInode::readdir_at` must consume a filesystem-owned directory-stream bridge, must not reach into raw `ExfatFs` fields from `inode.rs`, and must keep opened-inode reuse owned by `ExfatFs`.[`01_designer_core.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md#L18) [`01_designer_core.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md#L21) [`01_designer_core.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md#L75) [`01_designer_core.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md#L79) [`01_designer_core.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md#L98) [`01_designer_core.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md#L123)
- The async spec further says each call owns only its local record-stream walk, fresh `DirectoryEngine` creation is acceptable, and no hidden shared mutable scanner should survive across calls.[`02_designer_async.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/02_designer_async.md#L28) [`02_designer_async.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/02_designer_async.md#L33) [`02_designer_async.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/02_designer_async.md#L54) [`02_designer_async.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/02_designer_async.md#L62)
- The current implementation already matches that shape: `inode.rs` upgrades `Weak<ExfatFs>` per call, then uses `fs.directory_stream(...)` and `fs.resolve_or_publish_child_inode(...)`; the bridge itself lives in `fs.rs` and is the one creating `DirectoryEngine` and publishing canonical children.[`inode.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs#L273) [`inode.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs#L335) [`fs.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs#L394) [`fs.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs#L418)
- That split also matches the VFS contract more closely than a separate stream owner would: `Inode` is where `lookup()` and `readdir_at()` belong, while `FileSystem` remains the place for shared canonicalization and opened-inode reuse.[`inode.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/ext2/inode.rs#L207) [`inode.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/ext2/inode.rs#L631) [`inode.rs`](/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs#L313) [`inode.rs`](/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs#L329)

## Main Tradeoff

- Keeping `directory_stream` in `ExfatFs` preserves one owner for shared state, canonicalization, and canonical child publication, which avoids inventing a second service boundary.
- The cost is that `ExfatInode` has to bounce through `owner_fs()` on every directory op, so the code reads a little more indirect than an inode-local stream wrapper would.
- That indirectness is the right tradeoff here because the wrapper would otherwise become a second owner in practice, especially once `lookup()` needs `UpcaseTable` and canonical child reuse from `ExfatFs`.

## Recommendation

- Keep the current owner shape.
- If a follow-up is needed after the bugfix, make it a narrow creator cleanup only: a private inode-local helper may factor the repeated `directory_stream(...)` call shape, but it should remain a wrapper over the filesystem-owned bridge, not a new stream owner or cache owner.
