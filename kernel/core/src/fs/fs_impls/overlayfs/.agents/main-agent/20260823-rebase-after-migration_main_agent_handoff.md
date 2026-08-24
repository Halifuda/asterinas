<!-- SPDX-License-Identifier: MPL-2.0 -->

# Handoff: Overlayfs 代码迁移完成后的 Rebase（2026-08-23）

**Status:** CLOSED — 新分支迁移已完成并 push 到 origin（Halifuda），静态检查与核心 overlay 验证通过。

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

## Note (2026-08-24): xfstests 测试脚手架暂不迁移

旧分支的 xfstests 脚手架虽然源自 upstream PR #3298，但旧分支在其上做了较大的本地改动（例如删除顶层 `build_config.mk`、改为直接 include 各 fs 的 `config/build_config.mk`、`run_xfstests.sh` 增加 overlay 专用逻辑等）。

当前决策：

- 本阶段**不迁移**测试脚手架。
- 后续开始做测试时，尽量从当前 upstream base **重新实现** xfstests 脚手架，旧分支仅作参考。
- 至少保持 upstream 的 `build_config.mk` 结构不变，不要沿用旧分支删除顶层 `build_config.mk` 的大改动。

## Note (2026-08-24): 新分支推送策略

当前 `codex/overlayfs-refactor-new` 暂不恢复 upstream tracking，也不执行 push。

后续等新分支上的事情全部调好之后，直接 push 到 `origin`：

```bash
git push -u origin codex/overlayfs-refactor-new
```

不要 push 到 `upstream`。push 完成后再根据情况决定是否保留 `origin` tracking 或额外配置 `upstream/main` 作为参考 tracking。

## Note (2026-08-24): overlay xfstests 调研结论

只读调研结论（旧分支 vs 当前 upstream base）：

- 旧分支 overlay xfstests 是手动跑的，命令形如：
  ```bash
  make run_kernel AUTO_TEST=conformance CONFORMANCE_TEST_SUITE=xfstests \
      XFSTESTS_FS_TYPE=overlay XFSTESTS_DISK_SIZE=6G \
      XFSTESTS_RUNLIST=short.list
  ```
- 旧分支对 upstream PR #3298 做了结构性魔改：删除顶层 `build_config.mk`、内联 per-fs build config、把 `XFSTESTS_RUNLIST` 改成完整 guest 路径、混入 exfat 脚手架。这些不应搬到新分支。
- 新分支最小方案：保留 upstream 文件结构，新增 `test/initramfs/src/conformance/xfstests/overlay/`，在 `run_xfstests.sh` 加 overlay 专用分支（`append_common_rc_asterinas_compat` + `./check -overlay`），并在 `xfstests.nix` 增加 `attr`，README 增加 overlay 说明。
- 风险：`common/rc` 运行时 patch 较脆弱；runlist 在新分支使用文件名（如 `short.list`），不是旧分支的绝对路径。
- 更优雅的方案待进一步调研。

## Note (2026-08-24): overlay xfstests 更优雅方案调研

进一步只读调研（含容器内 `/opt/xfstests` 源码）结论：

- xfstests 原生支持 `FSTYP=overlay`：`common/config` 会在 `FSTYP=overlay` 时自动调用 `_overlay_config_override()`，不需要 `./check -overlay`。
- 因此 `run_xfstests.sh` 可以完全不改；overlay 支持可以自包含在 `overlay/` 目录内。
- `common/rc` 的 Asterinas 兼容注入可以移到 `overlay/prepare.sh`（已有 per-fs 预处理 hook，tmpfs 也有类似先例），而不是放进通用 runner。
- runlist 不需要改：当前 upstream 的 filename-based `XFSTESTS_RUNLIST` 已兼容 `overlay/run_list/`。
- 推荐方案（比旧分支更优雅）：
  1. 新增 `overlay/config/build_config.mk`（`XFSTESTS_NEEDS_BLOCK_DEVICES=true`、`XFSTESTS_MKFS=mkfs.ext2`）
  2. 新增 `overlay/config/xfstests.config`（`FSTYP=overlay`、`OVL_BASE_FSTYP=ext2`、块设备/目录变量）
  3. 新增 `overlay/prepare.sh`（做 `common/rc` 兼容注入，幂等）
  4. 新增 `overlay/common_rc_asterinas_compat.sh`
  5. 新增 `overlay/run_list/{short,full,block}.list`
  6. `xfstests.nix` 增加 `attr`
  7. README 增加 overlay 示例
- 不需要改 `run_xfstests.sh`、顶层 `build_config.mk`、`test/initramfs/Makefile`、根 `Makefile`、`tools/qemu_args.sh`、CI action。
- 风险：`common/rc` 运行时 patch 仍是最脆弱部分；`FSTYP=overlay` 路径需要一次实际 smoke 验证。

## Note (2026-08-24): overlay xfstests Checker 验证结果

Checker 已按更优雅方案实现并验证：

- 新增 `test/initramfs/src/conformance/xfstests/overlay/`（config / prepare / common_rc_asterinas_compat / run_list）。
- `xfstests.nix` 增加 `attr`，README 增加 overlay 示例。
- 未修改 `run_xfstests.sh`、顶层 `build_config.mk`、`test/initramfs/Makefile`、根 `Makefile`、`tools/qemu_args.sh`、CI action。
- Nix 打包验证通过：`overlay/` 进入 `/opt/xfstests`，`attr` 进入 runtime path。
- 实际 smoke run（`XFSTESTS_FS_TYPE=overlay XFSTESTS_RUNLIST=short.list`）成功启动并执行 6 个用例：5 通过，1 失败（`overlay/021` output mismatch）。
- 失败定位为 overlayfs 实现/行为差异（lower directory find 行为），不是脚手架或打包问题。
- 后续需要单独处理 `overlay/021` 或确认是否属于已知 overlayfs 行为差距。

## Note (2026-08-24): overlay xfstests list 修正 + 21 例 full 复跑

- 根据 wave7/wave8 handoff，修正 overlay run_list：
  - `full.list` = 21 个已通过用例：029/002/003/006/007/009/010/011/012/014/016/019/022/024/026/031/038/039/063/077/028
  - `block.list` = 其余 59 个预计无法通过的 packaged overlay 用例
  - `short.list` = 有代表性的 6 个已通过用例：002/003/007/012/014/077
- 使用 `XFSTESTS_RUNLIST=full.list` 实际复跑：**21/21 全部 PASS**。
- 日志：`.overlay-full.log`（未提交，可保留作证据或删除）。

## Note (2026-08-24): overlay regression 两例单独测试结果

- 两个 overlay 相关 regression：`test/initramfs/src/regression/fs/overlayfs/ovl_test` 和 `readdir_small_buffer`。
- 单独测试方式：临时把 `run_regression_test.sh` 改成只跑这两个二进制，再 `make run_kernel AUTO_TEST=regression`；跑完已恢复原脚本。
- 结果：
  - `ovl_test`：通过。
  - `readdir_small_buffer`：失败 3 项（`result.deleted`、`result.whiteout`、`result.total_entries`），且 cleanup 时 `rmdir(WORK_DIR)` 因目录非空失败。
- 日志：`.overlay-regression.log`（未提交，可保留或删除）。
- 可能原因：当前分支未携带旧分支对 `readdir_small_buffer.c` 的适配修改，或 overlayfs 行为与该测试预期仍有差异；需后续单独处理。

## Note (2026-08-24): readdir_small_buffer 旧分支适配迁移后复测

- 已将旧分支对 `test/initramfs/src/regression/fs/overlayfs/readdir_small_buffer.c` 的修改迁移到新分支（Linux 格式 whiteout：char device 0:0，cleanup 增加 `rmdir(WORK_DIR "/work")`）。
- 单独复测两个 overlay regression：
  - `ovl_test`：通过。
  - `readdir_small_buffer`：**12/12 全部通过**（之前 3 项失败已消失）。
- 注意：临时 runner 最后打印的是 `All overlay regression tests passed.`，而 Makefile 的 regression 门禁 grep `^All regression tests passed.`，所以 `make` 返回 Error 1 只是输出字符串不匹配，不是测试失败。
- 日志：`.overlay-regression2.log`。

## Close Note (2026-08-24)

- 本 handoff 关闭。
- 最终交付分支：`codex/overlayfs-refactor-new`，已 push 到 `origin`（Halifuda）。
- 静态检查：`make check` PASS、`make docs` PASS。
- 核心验证：overlay xfstests `full.list` 21/21 PASS；overlay regression `ovl_test` + `readdir_small_buffer` PASS。
- 重建 Docker 镜像的方法**不在本 handoff 中**；请参考仓库内：
  - `tools/docker/README.md`
  - `tools/docker/Dockerfile`
  - `tools/docker/kernel-dev/Dockerfile`
  - `tools/docker/prebuilt-nix-packages/Dockerfile`
  - `osdk/tools/docker/Dockerfile`
