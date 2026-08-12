<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: 2026-08-12 PR Draft Prep

**Date / Time:** 2026-08-12 10:29 CST（更新 2026-08-12：commit 切分改用 tokei 代码行数，忽略注释）
**Status:** `ACTIVE — PR 流程（wave-pr）：从 pure upstream main 切干净分支，交付「新 overlayfs + 配套 VFS 修改 + legacy 删除」，按 8-commit（tokei Code 1k~2k/commit）切分。上一份活跃 handoff（wave8）已 CLOSED（20260811-wave8-format-lint_main_agent_handoff.md）。`

## 1. Global State Pointer

- **上游基线:** `upstream/main` = `94a8f624d`（本 dev 分支 merge-base 即此 commit；本地已有，无需拉取）。
- **dev 分支:** `codex/overlayfs-refactor` @ `13243d6b7`（wave8 CLOSED；20 例可调度矩阵全量复测 PASS，user-confirmed 2026-08-11）。
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

## 3. Commit 切分方案（tokei Code 口径，2026-08-12 重做）

**结构事实（read-only 依赖分析，不变）:** 新 overlayfs 六组件是一个**强连通分量**（mount↔projection↔readdir_index↔copyup↔dir↔metadata_security 互相引用），不存在"每加一个组件就声明并编译"的 1~2k 拆分。**采用 story-first（先积木后合龙）：** C2–C7 只把文件加入树、不写入 `overlayfs/mod.rs` 的 `mod` 声明（惰性文件，`cargo check` 仍绿）；C8 一次性声明全部模块、切 `init()` 注册、删除 legacy。这与 dev 分支自身历史一致（Wave1–4 无编译 preflight，Wave5 首次编译）。

| # | Commit 主题 | 文件（`kernel/src/fs/` 下） | tokei Code | Code+Blank |
|---|---|---|---|---|
| C1 | VFS groundwork（纯 additive） | `vfs/fs_apis/{inode,inode_ext,registry}.rs`、`vfs/path/dentry.rs` | +56/−6（62） | ~70 |
| C2 | overlayfs mount（整组件） | `fs_impls/overlayfs/mount/{mod,options,policy,superblock,build,claims,layers}.rs` | 1,266 | 1,405 |
| C3 | projection ① 身份/查找核心 | `fs_impls/overlayfs/projection/{mod,identity,lower_id,entry,inode_cache}.rs` | 976 | 1,064 |
| C4 | projection ② inode + readdir 索引 | `fs_impls/overlayfs/projection/{inode,binding_cache}.rs`、`readdir_index.rs` | 1,131 | 1,258 |
| C5 | copyup | `fs_impls/overlayfs/copyup/{mod,coordination,trigger,workdir,promote}.rs` | 776 | 853 |
| C6 | dir（创建/链接/whiteout/删除/改名） | `fs_impls/overlayfs/dir/{mod,create,link,whiteout,remove,rename}.rs` | 1,202 | 1,271 |
| C7 | metadata_security | `fs_impls/overlayfs/metadata_security/{mod,metadata,permission,xattr}.rs` | 580 | 634 |
| C8 | **合龙**：mod.rs 声明全部模块 + `AccessType` + 注册切换；删 upstream `fs.rs`；删 `DirentCounter` | `fs_impls/overlayfs/mod.rs`、`fs_impls/overlayfs/fs.rs`(D)、`utils/{dirent_visitor,mod}.rs` | 1,295 changed（净 −1,271） | ~1,510 |

- 新 overlayfs 代码合计 Code = 5,945（C2–C7 5,931 + mod.rs 14）✓；全 PR Code 净变化 ≈ +5,945 + 62 − 30 − 1,259 ≈ **+4,718**。
- 除 C1（62，自洽 VFS 奠基）外，主 commit 均在 ~0.6k–1.4k Code；C3/C5/C7 略低于 1k Code（更小更好审，符合领导"小 commit 方便"的意图）。若要求每 commit ≥1k Code，可选合并：C5+C7 = 1,356（"文件内容与元数据策略"），或 C1 并入 C2 = 1,328。
- **备注:** 这是交付形态切分，非 `.agents` 协议里的 meso/micro pass 切分；`PASS_SLICING.md` 的 Creator/Checker pass 账目不受影响。

## 4. Open Decisions（发 PR draft 前需确认）

1. **行数口径**：默认以 tokei Code（忽略注释）为准（本 handoff §3 已按此重做）；如领导按"diff 总行数"看，可回看 11-commit 旧表（本文件上一版，已覆盖）。
2. **C1 是否并入 C2**：默认独立（VFS 先行更干净）；如需 ≥1k 可并入（→1,328 Code）。
3. **C3/C5/C7 低于 1k Code**：默认保持（更小更好审）；如需全部 ≥1k，C5+C7 合并（1,356 Code）。
4. **`.agents/` 不进 PR**：默认按 user 口径排除；确认无异议。
5. **每 commit 编译语义**：默认 story-first（中间 commit 惰性、C8 合龙编译）；如领导要求"每 commit 新代码参与编译"，需回到 §3 的取舍（整树一个 ~5.9k Code commit 或 stub 脚手架）并向 user 汇报。
6. **PR 是否附带运行证据**：xfstests framework 不进 PR，运行验证留在 dev 侧；PR draft 只带编译/静态门说明。

## 5. Next Actions（续任者第一动作）

1. 从 `upstream/main`（`94a8f624d`）切干净分支（`codex/pr-overlayfs-refactor` 或类似命名）。
2. 按 §3 表做**机械搬运**：`git diff 94a8f624d..13243d6b7 -- <路径>` + `git restore`/`git checkout` 逐组落地；唯一人为编辑 = C8 的 `mod.rs`（最终形态即 dev 分支现有 `mod.rs`，无需改写）。
3. 每 commit 门：`cargo check -p aster-kernel --target x86_64-unknown-none`（C2–C7 惰性通过）；C8 后跑 `cargo check` + `cargo clippy`（plain + `-Dwarnings`）+ `cargo fmt --check` + `git diff --check`（树与 dev HEAD 逐字节一致，wave8 已全绿，预期零修复）。
4. 提交 C1–C8 后，向 user 汇报 commit 清单（含每 commit tokei Code 行数），确认后再开 PR draft。
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
