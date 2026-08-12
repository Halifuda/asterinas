<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: 2026-08-12 PR Draft Prep

**Date / Time:** 2026-08-12 10:29 CST（更新 2026-08-12：commit 切分改用 tokei 代码行数，忽略注释）
**Status:** `ACTIVE — PR 流程（wave-pr）：从 pure upstream main（b96bbe3a1）切干净分支，交付「新 overlayfs + 配套 VFS 修改 + legacy 删除」，按 7-commit（tokei Code 1k~2k/commit）切分；VFS 修改按「首次使用点」均摊进对应生产 commit，无独立 VFS groundwork commit（2026-08-12 user 决策）。上一份活跃 handoff（wave8）已 CLOSED。`

## 1. Global State Pointer

- **上游基线:** `upstream/main` = `b96bbe3a1`（最新；dev 分支已 rebase 到其上，见 §7；含 kernel 树重构 `kernel/src` → `kernel/core/src`）。
- **dev 分支:** `codex/overlayfs-refactor` @ rebase 后 `cbca389bf`（含 reloc commit；wave8 CLOSED；20 例可调度矩阵全量复测 PASS，user-confirmed 2026-08-11）。
- **PR 前提（user 口径，2026-08-12）:** 领导偏好小 commit（1k~2k 行）；PR 只含「我们的新 overlayfs + 配套 VFS 修改 + 对 legacyfs 的删除」，**不带** xfstests CI framework 与 exfat refactor。
- **行数口径（2026-08-12 更新）:** commit 尺寸按 **tokei Code 行（忽略注释）** 统计；同时附 Code+Blank 作参考。工具：host `/home/ayd/.cargo/bin/tokei` 14.0.0。
- **Blueprint Updates Made:** 是 —— `SYSTEM_BLUEPRINT.md` Phase 4 追加 Wave7/Wave8 关闭与 PR 阶段指针（2026-08-12）。`PASS_SLICING.md` 未改（PR 切分是交付形态切分，不是 meso/micro pass 边界）。

## 2. PR 内容范围（只读统计，diff = `94a8f624d..13243d6b7`）

| 类别 | 文件（`kernel/src/fs/` 下） | 总行数 | tokei Code | Code+Blank |
|---|---|---|---|---|
| 新 overlayfs 代码（31 个新 `.rs`，不含 `.agents/`/legacy） | `fs_impls/overlayfs/` | +12,061 | **5,945** | 6,502 |
| 配套 VFS/utils 修改（6 文件） | `vfs/fs_apis/{inode,inode_ext,registry}.rs`、`vfs/path/dentry.rs`、`utils/{dirent_visitor,mod}.rs` | +73/−37 | **+62/−30** | ~+70/−35 |
| legacy 删除 | upstream `fs_impls/overlayfs/fs.rs`（1,610 行）+ `mod.rs` 切换 | −1,610 | **−1,259** | −1,480 |

- **明确排除:** `.agents/`（14,637 行 dev 协议/文档）、xfstests CI（`test/initramfs/…/xfstests/**`、`Makefile`、`tools/qemu_args.sh`、`.github/**`、`test/initramfs/Makefile`、`xfstests.nix`）、exfat refactor（`fs_impls/exfat/**`）、`.gitignore`/`.vscode`/`.codex`/skills。
- VFS 改动全部 additive / visibility-widening（`Extension::group3`、`OverlayInuseSlot`、`task_ctx()`、dentry 可见性放宽、`DirentCounter` 删除——仅 legacy 使用），不依赖 exfat，pure upstream 可编译。

## 3. Commit 切分方案（tokei Code 口径，2026-08-12 决策：VFS 按首次使用点均摊 → 7-commit）

**结构事实（read-only 依赖分析，不变）:** 新 overlayfs 六组件是一个**强连通分量**（mount↔projection↔readdir_index↔copyup↔dir↔metadata_security 互相引用），不存在"每加一个组件就声明并编译"的 1~2k 拆分。**采用 story-first（先积木后合龙）：** C1–C6 只把文件加入树、不写入 `overlayfs/mod.rs` 的 `mod` 声明（惰性文件，`cargo check` 仍绿）；C7 一次性声明全部模块、切 `init()` 注册、删除 legacy。这与 dev 分支自身历史一致（Wave1–4 无编译 preflight，Wave5 首次编译）。

**VFS 均摊决策（user 确认 2026-08-12）:** 无独立 VFS groundwork commit；每个 VFS 改动与其**首个生产消费方**同 commit（已逐一 grep 核实使用点）：
- `vfs/fs_apis/{inode,inode_ext,registry}.rs` + `dentry.rs` 的 `is_equal_or_descendant_of` 可见性 → **C1 mount**（仅 `mount/{claims,build,layers}.rs` 使用）。
- `dentry.rs` 的 `as_dir_dentry_or_err` + `DirDentry` 可见性 → **C2 projection ①**（首个消费方 `projection/entry.rs`）。
- `utils/{dirent_visitor,mod}.rs` 的 `DirentCounter` 删除 → **C7 合龙**（仅 legacy `fs.rs` 使用，须与删除同 commit）。
所有 VFS 改动 additive/清理、各自独立可编译；路径均为 rebase 后的 `kernel/core/src/fs/`。

| # | Commit 主题 | 文件（`kernel/core/src/fs/` 下） | tokei Code 约 |
|---|---|---|---|
| C1 | mount（整组件）+ 其 VFS 扩展点 | `fs_impls/overlayfs/mount/{mod,options,policy,superblock,build,claims,layers}.rs` + `vfs/fs_apis/{inode,inode_ext,registry}.rs` + `vfs/path/dentry.rs`（`is_equal_or_descendant_of` 一行） | ~1,330 |
| C2 | projection ① 身份/查找核心 + dentry 其余可见性 | `fs_impls/overlayfs/projection/{mod,identity,lower_id,entry,inode_cache}.rs` + `vfs/path/dentry.rs`（`as_dir_dentry_or_err`+`DirDentry` 两行） | ~980 |
| C3 | projection ② inode + readdir 索引 | `fs_impls/overlayfs/projection/{inode,binding_cache}.rs`、`readdir_index.rs` | 1,131 |
| C4 | copyup | `fs_impls/overlayfs/copyup/{mod,coordination,trigger,workdir,promote}.rs` | 776 |
| C5 | dir（创建/链接/whiteout/删除/改名） | `fs_impls/overlayfs/dir/{mod,create,link,whiteout,remove,rename}.rs` | 1,202 |
| C6 | metadata_security | `fs_impls/overlayfs/metadata_security/{mod,metadata,permission,xattr}.rs` | 580 |
| C7 | **合龙**：mod.rs 声明全部模块 + `AccessType` + 注册切换；删 upstream `fs.rs`；删 `DirentCounter` | `fs_impls/overlayfs/mod.rs`、`fs_impls/overlayfs/fs.rs`(D)、`utils/{dirent_visitor,mod}.rs` | 1,295 changed（净 −1,271） |

- 新 overlayfs 代码合计 Code = 5,945 ✓；全 PR Code 净变化 ≈ +5,945 + 70 − 30 − 1,259 ≈ **+4,726**。
- 主 commit 均在 ~0.6k–1.3k Code；C2/C4/C6 略低于 1k Code（更小更好审，符合领导"小 commit 方便"的意图）。若要求每 commit ≥1k Code，可选合并：C4+C6 = 1,356（"文件内容与元数据策略"）。
- **备注:** 这是交付形态切分，非 `.agents` 协议里的 meso/micro pass 切分；`PASS_SLICING.md` 的 Creator/Checker pass 账目不受影响。

## 4. Open Decisions（发 PR draft 前需确认）

1. **行数口径**：以 tokei Code（忽略注释）为准（§3 已按此）。
2. **VFS 均摊**：已按首次使用点分摊（C1/C2/C7），无独立 VFS commit（user 已确认）。
3. **C2/C4/C6 低于 1k Code**：默认保持（更小更好审）；如需全部 ≥1k，C4+C6 合并（1,356 Code）。
4. **`.agents/` 不进 PR**：默认按 user 口径排除；确认无异议。
5. **每 commit 编译语义**：默认 story-first（中间 commit 惰性、C7 合龙编译）；如领导要求"每 commit 新代码参与编译"，需回到 §3 的取舍（整树一个 ~5.9k Code commit 或 stub 脚手架）并向 user 汇报。
6. **PR 是否附带运行证据**：xfstests framework 不进 PR，运行验证留在 dev 侧；PR draft 只带编译/静态门说明。

## 5. Next Actions（续任者第一动作）

1. 干净分支已就绪：`~/asterinas-pr` `codex/pr-overlayfs-refactor` @ `b96bbe3a1`。
2. 按 §3 表做**机械搬运**：从 dev 工作区（rebase 后 HEAD `cbca389bf`，路径已与 upstream 一致）`git restore`/`git checkout` 到 PR 分支逐组落地；唯一人为编辑 = C7 的 `mod.rs`（最终形态即 dev 分支现有 `mod.rs`，无需改写）。
3. 每 commit 门：`cargo check -p aster-core --target x86_64-unknown-none`（C1–C6 惰性通过，VFS 部分真实编译）；C7 后跑 `cargo check` + `cargo clippy`（plain + `-Dwarnings`）+ `cargo fmt --check` + `git diff --check`（树与 dev HEAD 逐字节一致，全绿预期）。
4. 提交 C1–C7 后，向 user 汇报 commit 清单（含每 commit tokei Code 行数），确认后再开 PR draft。
5. 可选：把 20 例全量复测日志补录到 `run_evidence/`（user-confirmed 已通过；非阻塞）。

## 6. PR Comment Audit（2026-08-12，已完成）

- **执行**：5 个 Reviewer 子代理并行（V1 Direct Spawn Lane，fork_context=false；首轮因 model override 触达 provider 错误，去掉 override 后重派全部完成）：mount+mod / projection / copyup+readdir / dir / metadata_security+VFS/utils。packet：`subagent-tasks/pr-comment-audit-20260812/`；报告：`components/pr-comment-audit-20260812/pr_comment_audit_{mount,projection,copyup_readdir,dir,security_vfs}_20260812.md`；汇总：`pr_comment_audit_consolidated_20260812.md`。
- **结果**：共 **~139 处 findings**（BANNED_TERM ~85 / NOT_SELF_EXPLANATORY ~46 / STALE_OR_DUPLICATE ~8），主代理独立 grep 逐族核实为真实命中。主要家族：D 编号（D3–D33）、finding/cluster/objective ID（T1–T4、Objective 1/2、C1/C2、F1–F3、Change 1）、micro ID（P0-02/P0-16/P1-19）、Stage/Branch 标签、process marker（wave/census/accepted/recorded/frozen/insertion point 等）、Architect jargon（carrier/seam/credential-swap）、dir/ 锁域缩写无展开、stale TODO 与重复注释；另有 `projection/inode_cache.rs:256` runtime log 字符串泄露 `F2` 内部 ID（非注释，建议同批清理）。
- **干净文件**：`overlayfs/mod.rs`、`mount/options.rs`、`projection/{identity,lower_id,entry}.rs`、`copyup/workdir.rs`、全部 6 个 VFS/utils 文件。
- **修复波（2026-08-12，已完成并验收）**：5 个并行 Creator（mount 66 / projection 71 / copyup+readdir 29 / dir 35 / security 20 edits）+ 1 个 dir 残留续修（29 edits，清除注释中全部 carrier/seam）——仅注释/字符串文本/`#[expect]` reason 变化，零行为变化。
  - 主代理结构验收：`git diff -U0` 共 858 个变更非空行，**0 处非注释/字符串/expect 行**；`git diff --check` CLEAN。
  - 独立验收 grep：注释+字符串+expect reason 对 banned 家族（D/T/F/Objective/Change/P-xx/Branch/Stage/wave/census/insertion point/deferred decision/recorded VFS gap/credential-swap/minimize-nesting/pre-extraction/mount_resource_policy/option A 等）**0 命中**；`carrier`/`seam` 注释 0 命中（仅代码标识符保留 20 处，属既定例外）。
  - 容器门（`codex-asterinas-dev`）：`cargo check` PASS、clippy plain PASS、`RUSTFLAGS="-Dwarnings"` PASS、`cargo fmt --check` PASS、`git diff --check` CLEAN。
  - 策略：`components/pr-comment-audit-20260812/pr_comment_fix_strategy_20260812.md`（rev B）；Creator 报告：`pass_54_pr_comment_cleanup_{mount,projection,copyup_readdir,dir,security,dir_residual}_creator.md`。
  - **PR 的 comment 面已满足「无开发术语、自限自解释」**；待 commit（user 暂不 commit）。

## 7. Rebase 到最新 upstream main（2026-08-12，已完成）

- **新基线**：`upstream/main` = `b96bbe3a1`（领先原基线 `94a8f624d` 26 commits，含 kernel 树重构：`kernel/src/fs` → `kernel/core/src/fs`，crate `aster-kernel` → `aster-core`，FixedStr 移到 `aster_util` 等）。
- **执行**：`git rebase upstream/main` 48/48 全部重放；仅 3 处冲突（exfat 目录迁移、overlayfs `fs.rs`→`legacy_fs.rs` rename/rename、`utils/mod.rs` modify/delete），均机械消解（新文件移到 `kernel/core/...`、删除被 upstream 迁移的旧单文件、utils/mod.rs 取 upstream 新版 + 我们的 DirentCounter 单行改动）。
- **结构修复 commit** `cbca389bf`：overlayfs 实现（31 个 `.rs`）+ `.agents/` 整体迁到 `kernel/core/src/fs/fs_impls/overlayfs/`；`.gitignore` ignore 规则同步到新路径；旧路径 `kernel/src/fs` 清空；原 `git add -f` 跟踪的 subagent-tasks 文件与 4 个 `.gitkeep` 保持跟踪。
- **备份**：`.agents/` 被 ignore 内容（components/subagent-tasks/tmp，约 35MB）已备份到 `/tmp/ovfs_agents_ignored_backup_20260812.tar.gz`（7.2MB，1403 条目，含 run_evidence/receipts）。
- **编译门（容器）**：`cargo check -p aster-core` 0 errors / 0 warnings；`cargo check -p asterinas` PASS；clippy plain + `RUSTFLAGS="-Dwarnings"` PASS；`cargo fmt --check` PASS；`git diff --check` CLEAN。**无需 API 修复**（upstream 保留了 `utils::CStr256/Str16/Str64` 别名等）。
- **注意**：PR 8-commit 切分的文件路径全部变为 `kernel/core/src/fs/...`；`~/asterinas-pr` 的干净分支 `codex/pr-overlayfs-refactor` 已从 `b96bbe3a1` 切好。dev 分支未 push（ahead 75 / behind 48）。

## 8. PR CI Triage（2026-08-12，已完成诊断）

**上游 PR #3708 三个 CI fail 的根因定位**（x86-64 workflow run 31580563893 + Publish API Docs run 31580563958；head = `codex/pr-overlayfs-refactor` @ a0ac8baf）：

### 8.1 regression-test ×2（handover64-debug / multiboot2-smp4）— 同一根因
- 同一失败点：`test/initramfs/src/regression/fs/overlayfs/readdir_small_buffer.c`（**upstream 测试**，Tao Su 2026-04-17 `c1e6cfe44`，配套 legacy 修复 `9a71bf7b9`；**不在本 PR diff 内**）。两 job 断言完全一致：`result.deleted != 0`、`result.whiteout != 0`、`result.total_entries != 12`（实际 14 = `.`/`..` + 12 可见项，含泄漏的 `.wh.deleted` 与 `deleted`）。
- **根因 A（主）**：新 overlayfs 只按 **inode 标记** 识别 whiteout（char `0:0` 或 `trusted.overlay.whiteout` xattr，`projection/entry.rs::is_whiteout_inode`），**不识别 `.wh.` 名字前缀**。upstream 契约（legacy `legacy_fs.rs` + 该回归测试）把 upper 层任意 `.wh.*` 名视为 whiteout：`readdir` 合并视图必须隐藏 `.wh.deleted` 本身并隐藏 lower 同名 `deleted`。`readdir_index.rs` 的隐藏完全委托 `lookup_binding`（negative binding → skip），故修复点集中在 `projection/entry.rs`/`lookup_in_layers` 的 whiteout 判定（名字前缀 → negative binding）。
- **根因 B（次，同测试的 cleanup）**：`fatal error: cleanup_overlay_tree: rmdir(WORK_DIR) [Directory not empty]` —— 我们的 mount 按 Linux 风格在 workdir 内创建 `<workdir>/work` staging（`mount/claims.rs::prepare_workdir`），umount 后不删除；legacy 要求 workdir 空且不建子目录，因此上游测试 cleanup 的严格 `rmdir(WORK_DIR)` 失败（`ovl_test.c` 忽略 rmdir 返回值所以没挂）。需在 mount teardown 删除 `<workdir>/work`（VFS `FileSystem` 无 unmount hook，需 Drop 路径）或调整 staging 设计 —— 设计决策，待派包。
- 结论：user 判断正确，两个 regression CI 是同一个问题。

### 8.2 check_api_docs（Publish API Docs）— 5 处 rustdoc 错误（`RUSTDOCFLAGS="-Dwarnings --document-private-items"`，全 fatal）
1. `copyup/promote.rs:539` — `unresolved link: OverlayXattrPolicy::copy_eligible_xattrs`（`OverlayXattrPolicy` 未在 promote.rs 引入；方法存在于 `metadata_security/xattr.rs:379`）。修：import 或全路径。
2. `dir/remove.rs:422` — `unresolved link: crate::…::copyup::promote::CommitMarker`（`copyup/mod.rs` 中 `mod promote;` 私有，路径不可达）。修：`copyup` 根 re-export `CommitMarker` 或模块可见性放宽。
3. `projection/mod.rs:16` — `unresolved link: OverlayObjectId`（定义于 `projection/identity.rs`，未 re-export 到 mod 根）。修：加入 re-export 或改链接。
4. `copyup/workdir.rs:117` — `redundant explicit link target`（`mknod_object_type` 全路径冗余）。修：`[`mknod_object_type`]`。
5. `mount/mod.rs:41` — `redundant explicit link target`（`FileSystem::name` 全路径冗余）。修：`[`FileSystem::name`]`。

### 8.3 处置建议（下一动作）
- 8.1A 属行为语义修复：按协议走 bounded repair（Designer 会签 → Creator → Checker，参考 pass_45 先例），并本地/容器跑 `readdir_small_buffer` 复现验证。
- 8.1B 属设计决策（workdir staging 生命周期），与 8.1A 分开评估。
- 8.2 为机械 doc 修复（5 处），可并行处理；修后容器跑 `make docs`（RUSTDOCFLAGS 同 CI）验收。
- 修复前先确认 PR 分支 `~/asterinas-pr` 与 dev 分支的内容同步口径（PR head 对象不在本仓库，需在 PR 分支上改/搬运）。

### 8.4 补充：`.wh.` 前缀与 Linux 行为对照（2026-08-12，主代理核实）
- **Linux 主线从未使用 `.wh.` 前缀**：逐版本核对 `fs/overlayfs`（v3.18、v4.9、v4.14、v4.19、v5.15、本地 7.2.0-rc3），无 `WHITEOUT_PREFIX`/`\.wh\.`。Linux whiteout = **char 0:0 设备（目标同名）**，workdir 临时名创建后 rename 覆盖目标（3.18 起如此）；~6.7 起增加 **零长普通文件 + `trusted.overlay.whiteout` xattr**（`OVL_XATTR_XWHITEOUT`，docs: "never created by overlayfs"，供嵌套 overlay/用户态生成 lower），同样目标同名。
- `.wh.` 前缀实为 **AUFS / OCI 容器镜像层** 约定（docker/containerd 在应用层时把 `.wh.foo` 转换为 chardev 0:0 `foo`），不是内核 overlayfs 磁盘格式。
- 因此 upstream 回归测试 `readdir_small_buffer.c` 编码的是 **legacy Asterinas（AUFS/OCI 风格）** 语义，**不是 Linux 语义**：真实 Linux 会把 `.wh.deleted`（带内容普通文件）当作普通文件显示、且不隐藏 lower `deleted`。
- 我们的新实现（chardev 0:0 / xattr + 目标同名发布）**比 legacy 更贴近真实 Linux**，所以恰好过不了这个测试。
- 决策点（待 user）：(a) 加 `.wh.` 前缀读兼容（过测试、兼容 AUFS/OCI 风格层，但对 `.wh.*` 用户文件与 Linux 行为有差异）；(b) 保持纯 Linux 语义并推动 upstream 改测试（PR 保持红）；(c) 混合。主代理建议 (a)：识别为**叠加兼容**，自身仍产出 Linux 格式，行为是两者的超集。次要兼容 gap：我们 `is_whiteout_inode` 要求 xattr 值恰为 `b'y'` 且 1 字节，Linux 只要求零长文件 + xattr 存在（值不限）。

### 8.5 决策与回归测试盘点（2026-08-12，user 确认）
- **决策：不加 `.wh.` 前缀兼容，改测试。** 理由：我们行为自洽且贴合 Linux（whiteout = chardev 0:0 / `trusted.overlay.whiteout` xattr、目标同名；workdir 下建 `work/` 同 Linux `OVL_WORKDIR_NAME`）；`readdir_small_buffer.c` 是 legacy（AUFS/OCI `.wh.` 风格）契约。
- **回归测试 overlayfs 相关清单（全树 grep 确认，仅 2 个测试 + 基础设施）：**
  1. `test/initramfs/src/regression/fs/overlayfs/ovl_test.c`（upstream 起源 `764e3afa7`）— **不需要修改**（CI 已过；不涉 `.wh.`、不要求 workdir 空、cleanup 忽略 rmdir 返回值）。
  2. `test/initramfs/src/regression/fs/overlayfs/readdir_small_buffer.c`（upstream `c1e6cfe44`）— **需要修改两处**：
     - A. fixture 用普通文件 `.wh.deleted` 当 whiteout（legacy 语义）→ 改为 mount 后 `unlink(MERGED "/deleted")` 让内核产 Linux 格式 whiteout；cleanup 改 `unlink(UPPER "/deleted")`。断言不变（deleted==0 / whiteout==0 / total==12 含 `.`+`..`）。
     - B. cleanup 先 `rmdir(WORK_DIR "/work")` 再 `rmdir(WORK_DIR)`（Linux 同款残留，staging 平时为空）。
  3. 基础设施 `run_test.sh` / `fs/Makefile` — 不需要修改。
- 其余：regression 树无其他 overlay 引用；conformance 仅 gvisor blocklist 提到（与 overlayfs 无关）；xfstests overlay 套件是独立 lane。
