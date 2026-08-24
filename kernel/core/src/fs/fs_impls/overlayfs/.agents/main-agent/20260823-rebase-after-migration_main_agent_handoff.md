<!-- SPDX-License-Identifier: MPL-2.0 -->

# Handoff: Overlayfs 代码迁移完成后的 Rebase（2026-08-23）

**Status:** RECORDED ONLY — 代码迁移已完成，rebase 尚未执行。

## Goal

把当前 `codex/overlayfs-refactor` 分支 rebase 到最新的 `upstream/main`，并在 rebase 后完成必要的 API repair 和验证。

## 当前基线

- 当前分支：`codex/overlayfs-refactor`
- 最新 upstream：`upstream/main` = `604948581`
- 已 fetch，未在真实分支上执行 rebase
- 代码迁移已完成（overlayfs 新结构已落地）

## 已确认的上游变化

### 1. PR #3298 已合入 upstream

PR #3298 是 “Support multi file systems for `xfstests`”，upstream 已包含：

```text
aca673064 Support multi file systems for `xfstests`
b0aa4fb2c Integrate `tmpfs` into CI
```

它引入了：

- `XFSTESTS_FS_TYPE`
- `ext2/`、`tmpfs/`、`template/` 多文件系统 xfstests 配置目录
- `run_xfstests.sh` 改为按 `$XFSTESTS_FS_TYPE` 选择配置
- CI/Makefile 同步调整

### 2. VFS rename API 已变化

upstream 合入了 rename 重构：

```text
d921f38dd Pass resolved inodes through filesystem rename
f4a0ae777 Use cached inodes for ext2 rename
a0983c033 Use cached inodes for virtio-fs rename
36ae7fe10 Use cached inodes for exfat rename
```

`Inode::rename` 新签名：

```rust
fn rename(
    &self,
    old_name: &str,
    old_inode: &Arc<dyn Inode>,
    new_dir_inode: &Arc<dyn Inode>,
    new_name: &str,
    replaced_inode: Option<&Arc<dyn Inode>>,
    mode: RenameMode,
) -> Result<()>;
```

所有文件系统的 rename 实现都需要适配。

### 3. Docker 镜像版本已更新

upstream 把 CI 使用的 Docker 镜像从旧版本升级到了新版本：

```text
asterinas/asterinas:0.18.0-20260702   ->  asterinas/kernel-dev:0.18.1-20260805
asterinas/osdk:0.18.0-20260702        ->  asterinas/osdk-dev:0.18.1-20260805
```

涉及文件：

```text
.github/workflows/test_x86.yml
.github/workflows/test_xfstests_full.yml
.github/actions/test/action.yml
```

rebase 时这些镜像引用冲突应以 upstream 的新镜像为准。

### 4. 大量可见性收窄

upstream 把很多 `pub` 收窄为 `pub(crate)`，涉及：

- `kernel/core/src/fs/utils/dirent_visitor.rs`
- `kernel/core/src/fs/utils/mod.rs`
- `kernel/core/src/fs/vfs/fs_apis/inode.rs`
- `kernel/core/src/fs/vfs/fs_apis/inode_ext.rs`
- `kernel/core/src/fs/vfs/fs_apis/xattr.rs`
- exfat 多个文件

## Rebase 冲突清单

### A. CI / xfstests 基础设施（PR #3298 引入，真实冲突）

```text
.github/actions/test/action.yml
.github/workflows/test_x86.yml
.github/workflows/test_xfstests_full.yml
Makefile
test/initramfs/Makefile
test/initramfs/nix/conformance/xfstests.nix
test/initramfs/src/conformance/xfstests/README.md
test/initramfs/src/conformance/xfstests/ext2/config/xfstests.config
test/initramfs/src/conformance/xfstests/run_xfstests.sh
```

处理原则：**以 upstream 的 PR #3298 框架为准**，再重新应用我们本地 overlayfs 相关的 xfstests 配置/runner 修改。

### B. exfat（真实冲突，但和 overlayfs 无关）

```text
kernel/core/src/fs/fs_impls/exfat/dentry.rs
kernel/core/src/fs/fs_impls/exfat/fat.rs
kernel/core/src/fs/fs_impls/exfat/fs.rs
kernel/core/src/fs/fs_impls/exfat/inode.rs
kernel/core/src/fs/fs_impls/exfat/super_block.rs
kernel/core/src/fs/fs_impls/exfat/utils.rs
```

原因：我们分支有 exfat 重构，upstream 也改了 exfat（可见性收窄 + rename API）。

处理原则：需要单独决策——保留我们的 exfat 重构，还是以 upstream exfat 为准。不要混进 overlayfs rebase 的同一个 commit。

### C. fs/utils（真实冲突，但机械）

```text
kernel/core/src/fs/utils/dirent_visitor.rs
kernel/core/src/fs/utils/mod.rs
```

原因：upstream 做 `pub` → `pub(crate)` 收窄，我们分支也改过这些文件。

处理原则：合并可见性，确保 overlayfs 仍能访问所需符号；这类冲突较小。

### D. overlayfs/legacy_fs.rs（真实冲突，但是死代码）

```text
kernel/core/src/fs/fs_impls/overlayfs/legacy_fs.rs
```

原因：upstream 修改了旧 `overlayfs/fs.rs`，我们分支把它 rename 成 `legacy_fs.rs`。

处理原则：legacy 是死代码，保留我们的版本即可；如果后续不再需要，可以直接删除。

### E. VFS API（没有文本冲突，但 rebase 后编译会挂）

```text
kernel/core/src/fs/vfs/fs_apis/inode.rs
kernel/core/src/fs/vfs/fs_apis/inode_ext.rs
kernel/core/src/fs/vfs/fs_apis/xattr.rs
kernel/core/src/fs/vfs/path/dentry.rs
kernel/core/src/fs/vfs/path/mod.rs
kernel/core/src/fs/vfs/path/resolver.rs
kernel/core/src/fs/vfs/range_lock/*
```

处理原则：这些不是文本冲突，而是 API 迁移。rebase 后必须做一轮 API repair。

## Rebase 执行步骤

1. 确认代码迁移已全部 commit，工作区干净。
2. 创建备份分支：
   ```bash
   git branch codex/overlayfs-refactor-backup-20260823
   ```
3. 建议先在临时 worktree 试 rebase：
   ```bash
   git worktree add --detach /tmp/asterinas-rebase-preview HEAD
   cd /tmp/asterinas-rebase-preview
   git rebase upstream/main
   ```
   观察完整冲突列表后再回真实分支执行。
4. 在真实分支 rebase：
   ```bash
   git rebase upstream/main
   ```
5. 按顺序解决冲突：
   - 先处理 CI/xfstests（以 upstream 为准）
   - 再处理 fs/utils（机械合并）
   - 再处理 legacy_fs.rs（保留/删除）
   - exfat 单独决策，建议拆到独立 commit
6. 完成 rebase 后做 API repair：
   - 更新所有 `Inode::rename` 实现
   - 适配 `xattr` 新增的 `clear_file_priv` 等接口
   - 适配 `InodeWriter` 删除带来的调用点
   - 适配可见性收窄
7. 验证：
   ```bash
   cargo check -p asterinas --target x86_64-unknown-none
   make check
   make docs
   ```
8. 跑 overlayfs 相关测试 / xfstests。

## Next Action（给下一个 agent）

1. 先读本 handoff 和 `20260823-125917-structure-migration-kickoff_main_agent_handoff.md`。
2. 确认当前代码迁移已全部提交、工作区无未提交的迁移残留。
3. 创建备份分支。
4. 在临时 worktree 做一次完整 rebase 预览，记录所有实际冲突。
5. 回到真实分支执行 rebase，按本 handoff 的冲突分组逐一解决。
6. rebase 完成后优先修 VFS rename API 相关的编译错误。
7. 全部验证通过后再考虑是否删除 `old/` 或 `legacy_fs.rs`。

## Note (2026-08-24): `DirentCounter` cleanup

During the new-branch migration, `kernel/core/src/fs/utils/dirent_visitor.rs` and `kernel/core/src/fs/utils/mod.rs` were kept as upstream (with `pub(crate)` visibility and `DirentCounter`). The refactored overlayfs does **not** need `DirentCounter`; it should be removed in a later cleanup pass. This note is to prevent it from being treated as a required dependency of the new overlayfs.
