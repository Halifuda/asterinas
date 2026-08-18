<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: 2026-08-17 代码结构清理（恢复/重放 → 可见性收敛 → 后续 G 序列规划）

**Date / Time:** 2026-08-18 CST
**Status:** `ACTIVE — G8 机械包 + 六项方向修改均已 amend；run06 PASS；
G10 Review 完成；G10 P1 已执行（OPAQUE_MARKER_VALUE → pub(super)），
run07 PASS，待 amend；G7 规划态；G9 待 user`
**Branch:** `codex/overlayfs-refactor`（WIP 以 `--amend --no-edit` 方式维护，
父提交 `978754581` `wave-9: comments audit`；本 handoff 不引用自指 hash）

## 1. 当前状态指针

- **代码状态**: G2–G6 session 重放完成；C-07…C-12 编译修复完成；可见性审计 +
  机械替换完成并 amend 进 WIP；G8 六项方向修改落码并随本次 amend 合并进
  WIP。`cargo check -p asterinas --target x86_64-unknown-none` 最新 gate
  **run07 PASS（2026-08-18，exit 0，0 errors / 0 warnings，6.02s；覆盖
  G10 P1 收窄）**（run06 为方向项后 gate，亦 PASS）。
- **可见性终态**:
  - 长形式 `pub(in crate::fs::fs_impls::overlayfs)` = **0**。
  - 短形式 `pub(in overlayfs)` = **112**（全部为 KEEP；见 §4 全量清单）。
  - 本次实际收窄为 `pub(super)` = **101**（87 CASE2_EQUIV + 14 CASE1_NARROW；
    原 16 条 CASE1 中 2 条因跨模块签名泄露回退）。
  - 所有含短形式的文件均带 `#![short_vis_path::add(overlayfs)]`，属性检查
    0 缺失。
- **关键证据**:
  - 重放 session: `.agents/tmp/G2-6/`（8 个 JSONL，只读）。
  - 短形式审计:
    `components/code-structure-cleanup/visibility_super_audit_20260818/
    task_reviewer_overlayfs_visibility_super_audit_20260818_report.md`
    （56 = 28/3/25）。
  - 长形式审计:
    `components/code-structure-cleanup/visibility_super_audit_long_20260818/
    task_reviewer_overlayfs_visibility_super_audit_long_20260818_report.md`
    （157 = 59/13/85）。
  - 替换收据:
    `components/code-structure-cleanup/visibility_apply_20260818/
    task_creator_overlayfs_visibility_apply_20260818_receipt.md`。
  - 编译 gate: `components/code-structure-cleanup/checker_compile_20260818/
    run05_cargo_check/`（PASS）+ 收据
    `pass_code_structure_checker_compile_run05_20260818.md`。
  - 编译 gate（方向项后）: `.../run06_cargo_check/`（**PASS**，main-agent 直跑
    user 授权）+ 收据
    `pass_code_structure_checker_compile_run06_20260818.md`。
- **审计修正（重要）**: `copyup/promote.rs::CommitMarker` 与
  `projection/lower_id.rs::LowerIdRecord` 原判 `CASE1_NARROW`，但分别通过
  `OverlayInode::run_recipe` 与 `OverlayFs::read_lower_id` 等
  `pub(in overlayfs)` 接口跨模块暴露；run04 编译失败后已回退为 KEEP
  （短形式），故最终 KEEP=112、NARROW 实际落地=14。
- **G8 机械包（2026-08-18, user-directed）**：1 个
  `opencode-go` + `deepseek-v4-flash` Creator 经 workflow 直派执行 12 个
  机械 OBJ（S-52/S-53/S-36/S-39/S-63/S-41/S-42/S-45/S-49/S-54/S-60/S-67），
  收据 **12/12 闭合**（OBJ-12(f) 为 `done(pre-applied)`，锁序理由见收据）。
  - packet: `subagent-tasks/code-structure-cleanup/
    task_creator_overlayfs_g8_mechanical_20260818_dispatch.md`。
  - 收据: `components/code-structure-cleanup/g8_mechanical_20260818/
    task_creator_overlayfs_g8_mechanical_20260818_receipt.md`。
  - 主代理亲自验收：`git diff --check` clean；diff 恰为 write-set 20 个
    `.rs`；验收 grep 通过（entry 私有常量 0 残留、static
    `try_from_full_name(...).ok_or_else` 散射 0、`WHITEOUT_MARKER_VALUE`
    唯一权威、rename 二次 lookup 0）；另由主代理机械修复 1 处顺带未用
    import（`dir/create.rs::OverlayFs`）。
  - compile_preflight **withheld**（Creator 未跑任何命令）；编译门
    **run06 PASS（exit 0，main-agent 直跑）**，只 `cargo check`。
  - 收据记录 1 处 packet 内部签名偏差的机械修正：`entry.rs` 两调用点对
    `XattrPolicy::has_marker` 显式传 `&XattrPolicy`（ZST），行为/错误码/
    锁序不变。
- **G8 方向修改 Step 1/2（2026-08-18, user-directed）**：2 个
  `opencode-go` + `deepseek-v4-flash` Creator 顺序执行六项方向修改
  （S-29 按 user 裁定不派）。**未触发 opencode-go 周限额，无需切
  deepseek-official。**
  - Step 1 `task_creator_overlayfs_g8_direction_step1_20260818`：
    S-64/S-65/S-66/S-62(2,3 方案 A)，收据 4/4 done。
  - Step 2 `task_creator_overlayfs_g8_direction_step2_20260818`：
    S-59/S-61，收据 2/2 done。
  - 主代理亲自验收：`git diff --check` clean；diff 只含两 packet
    write-set 并集的 13 个 `.rs`；关键 grep 通过（`self.check_permission`
    in create/remove = 0；EROFS 双门消失；`same_layer_composition` 就位；
    `validate_workdir_against_lowers` 就位；`unique_temp_name`/serial/
    `AtomicU64` = 0；`self.run_recipe(` in promote = 0 且 `run_recipe`
    保留给 dir）。另由主代理机械修复 1 处顺带未用变量
    （`whiteout.rs::create_whiteout_temp` 的 `workdir_path`）。
  - 两 packet 均 compile withheld；**run06 PASS（main-agent 直跑，exit 0）**
    已验证方向项改动；**13 个 `.rs` 已随 user 指令 amend 进 WIP**。
  - 收据: `components/code-structure-cleanup/g8_direction_step1_20260818/`
    与 `.../g8_direction_step2_20260818/`。

## 2. 已完成的恢复/替换流水（不再执行）

1. 按 session 轨迹机械重放 G2→G3→G4→G4-finish→G5a→G5b→G6→G6-compile。
2. C-07 补 `try_update`；C-08/C-09/C-10 编译可见性修复。
3. 两轮 Reviewer 审计（短 56、长 157），只读，0 `.rs` 改动。
4. Flash Creator 执行 103 处收窄 + 85 处长→短 + 12 个属性补齐；amend WIP。
5. run04 发现 2 处审计错误（CommitMarker/LowerIdRecord），C-11/C-12 回退，
   再次 amend；run05 PASS。
6. **G1 验证（删除事故前已完成并验收，user-confirmed 2026-08-18）**：
   - S-46 = 方案 **a**：删 `project_new_upper` roundtrip，直接
     `self.replace_facts(...)`；`replace_facts` 改 `&self` 接收者或私有链传
     `Arc<Self>`。
   - S-51 = 方案 **a**：保留 lower 1-based 编号，显式
     `UPPER_LAYER_INDEX=0` / `LOWER_LAYER_INDEX_BASE=1` 常量 + slot-0 约定 doc。
   - 当时 0 `.rs` 改动、0 build/test；**两裁决尚未落码**，未指派到
     G8/G10/G7，待 user 指令单独派 Creator。
   - 对应 v2 报告：S-46=H-13、S-51=H-14。

## 3. 后续 G 序列（user-directed 2026-08-18；只规划，不执行）

顺序：**G8 → G10 → G7**。

### 3.0 G1–G9 S 编号总账（67 条，编号永不取消）

| G | S 编号 | 状态 | v2 报告映射 |
|---|---|---|---|
| G1 | S-46, S-51 | 已验证，未落码 | H-13, H-14 |
| G2 | S-07, S-09, S-08, S-48, S-14, S-28, S-13 | 已重放 | B-1, B-3, B-2, H-4, C-2, D-11, C-1 |
| G3 | S-01, S-30, S-03, S-02, S-35 | 已重放 | A-1, A-8, A-3, A-2, I-1 |
| G4 | S-15, S-16, S-17, S-12, S-18, S-19, S-04, S-05, S-43, S-44, S-47, S-55 | 已重放（含 S-04 续派） | C-3, C-4, C-5, B-6, C-6, C-7, A-4, A-6, A-9, H-9, H-3, H-7 |
| G5 | S-11, S-20, S-21, S-22, S-23, S-56, S-25, S-24, S-10, S-40, S-26, S-27, S-57, S-37, S-38 | 已重放（G5-a/G5-b） | B-5, D-1, D-2, D-3, D-4, D-9, D-6, D-5, B-4, A-5, D-7, D-10, D-13, H-10, H-11 |
| G6 | S-31, S-32, S-33 | 已重放 | E-1, E-2, E-3 |
| G7 | S-34, S-58 | 待派发 | F-1, F-2 |
| G8 | S-52, S-53, S-36, S-61, S-39, S-63, S-29, S-41, S-42, S-45, S-49, S-54, S-59, S-60, S-62, S-64, S-65, S-66, S-67 | 机械 12/12 已执行；7 项方向已裁决未落码（§3.1） | 见 G8 表 |
| G9 | S-06, S-50 | 未排序，待 user | A-7, H-6 |

> G2–G6 的映射来自各 session 中保存的原 packet 原文；G1 来自 user-confirmed
> 验证记录；G9 的 S-06=A-7 由排除法确定；G8 的逐条映射见下表（主代理重建
> 固定）。总账 67/67 闭合。

- **G8（去重与收敛批；19 条）** — 待派发。
  - **S 编号集合（原编号保留，不得取消/改写）**：
    `S-52 / S-53 / S-36 / S-61 / S-39 / S-63 / S-29 / S-41 / S-42 /
     S-45 / S-49 / S-54 / S-59 / S-60 / S-62 / S-64 / S-65 / S-66 / S-67`。
  - **映射来源说明**：原 supplement 执行顺序报告已丢失，无法恢复旧
    S→原文逐条文本；以下 19 条内容以恢复的 v2 审计报告为准，由主代理
    按旧 G8 顺序 + v2 执行位重建并固定。若 user 处有原 S 原文可覆盖，
    否则此映射即为权威。
  - **逐条内容（S → v2 报告条目 → 动作）**：

    | S | v2 | 内容 | 执行形态 |
    |---|---|---|---|
    | S-52 | G-6 | “目录 Path + 名字 → 子 Path”样板 ×8 → 共享 `lookup_child_path` | 机械 Creator |
    | S-53 | G-7 | link 目标发布的两步与 `create.rs` 私有发布相同 → dir 共享入口 | 机械 Creator |
    | S-36 | G-1 | opaque marker 写入两处重复 → `set_opaque_marker`（含 capability gate） | 机械 Creator（小 spec） |
    | S-61 | G-2 | `promote.rs` 四个 object-kind arm 同构 → 抽共享骨架 + 数据搬运闭包 | **设计优先** |
    | S-39 | G-3 | 两处“重观测 whiteout + 发布负 binding” → `publish_whiteout_binding`（依赖 S-52/G-6） | 机械 Creator |
    | S-63 | G-4 | whiteout/opaque 常量三处分散 → 统一归 `metadata_security/xattr.rs`（legacy 可见性约束） | 机械 Creator |
    | S-29 | G-5 | `lookup_in_layers` 上下两臂近似重复 → 参数化单层扫描，保留 merge-stop 差异 | **设计优先** |
    | S-41 | D-8 | “取 upper 层 + 构造 upper 子对象” ×3 → `upper_layer()` + `child_real_object()` | 机械 Creator |
    | S-42 | G-8 | 1 字节 marker 读谓词 ×3 → 参数化统一谓词（依赖 S-63/G-4） | 机械 Creator |
    | S-45 | G-9 | readdir 收集非 dot 子名 ×4 → 共享 helper | 机械 Creator |
    | S-49 | G-10 | `XattrName::try_from_full_name(CONST)...` 样板 ×22 → 每 marker 常量配 `name()` | 机械 Creator（面广） |
    | S-54 | G-11 | `publish_whiteout` 两个相同 rename 臂合并 | 机械 Creator |
    | S-59 | G-12 | workdir temp 两套命名（CSPRNG probe vs 组合名）统一 | **小设计** |
    | S-60 | H-1 | rename 对 `(parent, old_name)` 两次 `lookup_binding` → binding 传参消一次 | 机械 Creator（语义复核） |
    | S-62 | H-2 | dir 入口前导 5 处重复 + EROFS 门重复；双层 admission 契约 | **设计优先/裁决** |
    | S-64 | H-5 | `read_at_impl` 取两次 facts 快照 → 统一传已取快照 | **小设计** |
    | S-65 | H-8 | 两种 facts 相等比较器并存 → 提取并命名 `same_layer_composition` | **小设计（命名契约）** |
    | S-66 | D-12 | workdir-vs-lower overlap 谓词合并；校验时机属逻辑轮 | **设计优先/边界裁决** |
    | S-67 | H-12 | 行级微项包 (a)–(g)（impl 块合并、重复 clone、短路迭代等） | 机械 Creator |

  - **派发建议**: 先把「机械 Creator」项按依赖打包成 packet；「设计优先 /
    小设计」项先各立只读 Reviewer/Designer 提案（不改 `.rs`），批准后再
    开 Creator。不得一次混派设计与实现。
  - 执行前必须以 v2 报告对应条目为 packet 原文，禁止凭旧 S 号盲改。

### 3.1 G8 七项设计/裁决方向（user-confirmed 2026-08-18；仅方向，未落码）

> 逐条以 user 最新说明为准；执行时另立 Designer/Creator packet，禁止凭本节直接改码。

| S | v2 | 方向结论 |
|---|---|---|
| S-61 | G-2 | promote 四臂共享骨架；`run_recipe<T>` 的 `T` 只为外部 `create.rs`（`T=Arc<OverlayInode>`）存在，promote 四臂全是 `Result<()>`。**promote 内部直接做，不用数据搬运闭包**：按 object-kind 在内部 match 做数据搬运步，显式保留 committed-vs-precommit 错误分类（rename 已提交→reconcile；未提交→cleanup temp）。 |
| S-29 | G-5 | **整体不做（user 裁定）**。不派 Designer、不合并上下臂；S 编号保留，deferred。 |
| S-62 | H-2 | **只做第二、三点**。① 入口前导 ×5 不合并（user 认可分发 impl 重复）。② 删 `copyup/mod.rs` `resize_impl`/`fallocate_impl` 的外层冗余 EROFS 门（`check_local_permission` 在 copy-up 前已做同门，删除后行为逐字不变）。③ **方案 A（user 选定）**：删 `create_upper_only`/`create_over_whiteout`/`remove_target` 三处内层 `check_permission(Mutating, MAY_WRITE)`，并把“caller 已持 parent dir transaction lock 且已通过同检查”写成 recipe 前置契约。 |
| S-59 | G-12 | **理解修正（user 2026-08-18）**：保留 **CSPRNG 随机命名那套**为唯一命名工具（“第二套”= 随机命名；upstream serial 过强关切）；删除 `workdir.rs` 组合名 `workdir_temp_name` + `NEXT_TEMP_SERIAL`/`next_temp_serial` 原子 serial 机构；probe 前缀保留；`EEXIST` 8 次重试语义不变。 |
| S-64 | H-5 | **认可**。`read_at_impl` 改用同一份 facts：`let real = facts.select_real_inode();`（`ObjectFacts::select_real_inode` 已存在），双快照窗口与旧注释一并消除。 |
| S-65 | H-8 | **认可**。提取 `ObjectFacts::same_layer_composition`（值级 fsid+ino 层组成比较），`ensure_readdir_index` 复用它；doc 写清与 `same_visible_identity`（ptr 级 cache 身份）的适用边界，二者不可互换。 |
| S-66 | D-12 | **只摘 helper，不裁决时机**。把 `OverlayFs::new` 内 workdir-vs-lower 循环抽为私有 helper（workdir 专用 EINVAL 文案保留）；是否与 `validate_layer_overlap` 共用底层 dentry-overlap 谓词**可选**；校验时机/MT-21/22 仍留逻辑轮。 |

- **执行状态（2026-08-18）**：除 S-29（不派）外，其余 6 项已由 Step 1/2
  两个 Flash Creator 落码（S-62 只做 2/3 点；S-59 保留 CSPRNG 随机命名；
  S-61 promote 无闭包骨架；S-64/S-65/S-66 按上表）。详见 §1。
- compile gate run06 PASS，且方向项已 amend 进 WIP；G10/G7 仍为规划态。

- **G10（KEEP 归位研讨；本轮新增，user-directed）** — **已完成 2026-08-18（只读 Review，主代理结构验收通过）**。
  - **对象**: 当前树 active `.rs` 的全部 `pub(in overlayfs)`（排除
    `legacy_fs.rs`），权威清单 **121 行**（handoff §4 的历史 112 条口径已
    被 G8 机械+方向项新增的 9 条 cross-meso helper 更新；逐条见 G10 报告）。
  - **问题**: 逐条研讨“能否把条目/其属主移动到更合理的位置，使它可以
    变成 `pub(super)`，且不造成任何文件膨胀”。
  - **约束**（执行中全部遵守）: 不改生产 `.rs`；不制造文件膨胀；逐条
    disposition；不改行为/锁/持久化/错误码；不引入新实体；实际移动须另立
    Creator packet。
  - **Result**: **121/121 闭合** — 保持 KEEP 119；需更小范围调整 1
    （`OPAQUE_MARKER_VALUE` → `pub(super)`）；可移动 1
    （`UpperWorkdirClaim::workdir_workspace_path` → workdir.rs co-location，
    报告建议保守 KEEP）。方向项 helper/method 审计结论：
    `run_recipe` 不移入 dir；`workdir_temp_name` 保留 `#target#16hex` 不做
    纯随机简化；`finish_promotion` 9 参数维持私有骨架；收据 census 无遗漏。
  - **产出（当前权威 KEEP 清单文件，已确认存在）**:
    `components/code-structure-cleanup/visibility_keep_relocation_20260818/
    task_reviewer_overlayfs_g10_keep_relocation_20260818_report.md`
    （42.5 KB；§3-A.0 权威清单生成 + §3-A.1 逐条 disposition 表 =
    121 行全量清单，§3-A.2 统计；行号锚点以 P1 执行前的当前工作树为准）。
  - **下一步（P1 已执行）**: 2026-08-18 主代理直改
    `OPAQUE_MARKER_VALUE` `pub(in overlayfs)` → `pub(super)`（1 行），
    run07 PASS；当前 active KEEP 计数 **120**（清单文件仍为 121 行口径，
    其中该 1 行已 disposition 为“需小调整”并已执行）。P2
    （workdir_workspace_path co-location）未执行，报告建议保守 KEEP，
    仍待 user 最终确认。

- **G7（锁方向）** — 待派发，不提前实现。
  `S-34` → v2 **F-1**（`InodeInner` 粗粒度锁，仅方向）与 `S-58` → v2 **F-2**
  （`with_facts` 只读借用，与 S-34 二选一/配套）。实际改锁必须另立
  Designer/Creator packet。

- **G9** — 未列入上述顺序，保持待 user 指令。原编号保留：
  - `S-06` → v2 **A-7**（`readdir_index.rs` 顶层模块位置 keep 记录，无代码）。
  - `S-50` → v2 **H-6**（`inherit_methods_macro` 评估：`inode.rs` 约 180 行
    `Inode` trait 转发块用手写还是宏，裁决项）。

## 4. KEEP 全量清单（112 条历史口径；当前权威清单见 G10 报告）

> 本节 112 条是 G8 机械/方向项执行前的历史审计口径。**当前树（P1 已执行）的
> 权威 KEEP 清单文件为 G10 报告 §3-A**：
> `components/code-structure-cleanup/visibility_keep_relocation_20260818/
> task_reviewer_overlayfs_g10_keep_relocation_20260818_report.md`
> （121 行全量 disposition；其中 `OPAQUE_MARKER_VALUE` 已收窄，live 计数 120）。
> 历史 line 级锚点以上述两份审计报告为准（部分文件补 attr 后行号整体 +1；
> CommitMarker/LowerIdRecord 以当前文件为准）。

### 4.1 短形式审计 KEEP（25 条）

- `mod.rs`（1）: `AccessType`
- `mount/layers.rs`（13）: `LayerStack`、`RealPath`、
  `RealPath::{from_path, upgrade, inode}`、`Layer`、
  `Layer::{root_path, fs, fsid, container_dev_id}`、
  `LayerStack::{upper_layer, lower_layers, lower_layer_root_ino_for_origin}`
- `mount/policy.rs`（6）: `MountPolicy`、`is_effective_read_only`、
  `is_default_permissions`、`uuid`、`upper_capabilities`、
  `UpperFilesystemCapabilities`
- `mount/claims.rs`（3）: `Uuid`、`Uuid::value`、`UpperWorkdirClaim`
- `mount/build.rs`（1）: `OverlayFs::new`
- `mount/options.rs`（1）: `XinoMode`

### 4.2 长形式审计 KEEP（85 条）

- `metadata_security/xattr.rs`（13）: `XattrPolicy`、
  `OPAQUE_XATTR_FULL_NAME`、`OPAQUE_MARKER_VALUE`、
  `WHITEOUT_XATTR_FULL_NAME`、`XattrCopyPolicy`、`XattrPolicy::is_private`、
  `XattrPolicy::copy_eligible_xattrs`、`XattrPolicy::set_impure_marker`、
  `XattrPolicy::refresh_impure_marker`、`OverlayInode::get_xattr_impl`、
  `OverlayInode::set_xattr_impl`、`OverlayInode::list_xattr_impl`、
  `OverlayInode::remove_xattr_impl`
- `metadata_security/metadata.rs`（6）: `set_mode_impl`、`set_owner_impl`、
  `set_group_impl`、`set_atime_impl`、`set_mtime_impl`、`set_ctime_impl`
- `metadata_security/permission.rs`（1）: `OverlayInode::check_permission`
  （两参 AccessType 版）
- `projection/binding_cache.rs`（16）: `Binding`、`PositiveBinding`、
  `PositiveBinding::new`、`PositiveBinding::inode`、`PositiveKind`、
  `NegativeBinding`、`HiddenEvidence`、`HiddenEvidence::new`、`BindingKey`、
  `BindingKey::new`、`BindingCache`、`BindingCache::new`、
  `BindingCache::insert`、`BindingCache::invalidate`、
  `BindingCache::invalidate_parent`、`Binding::into_inode`
- `projection/identity.rs`（16）: `ObjectId`、`ObjectId::{dev, ino}`、
  `LowerLayerIdentity`、`LowerLayerIdentity::{fsid, container_dev_id,
  lower_layer_root_ino}`、`IdentityPolicy`、`IdentityPolicy::XINO_SHIFT`、
  `IdentityPolicy::new`、`is_xino_effective`、`is_all_layers_same_fs`、
  `project_object_id`、`project_object_id_from_lower_id`、
  `is_directory_projection_deterministic`、`resolve_layer_id_for_record`
- `projection/entry.rs`（10）: `is_whiteout_inode`、`RealObject`、
  `identity_only`、`from_layer_path`、`layer_index`、`real_inode`、
  `real_path`、`fsid`、`container_dev_id`、`is_opaque_directory`
- `projection/inode_cache.rs`（8）: `RealObjectKey`、`RealObjectKey::from_source`、
  `RealObjectKey::from_facts`、`InodeCache`、`InodeCache::new`、
  `InodeCache::get`、`InodeCache::rekey_keep_old_alias`、
  `InodeCache::get_or_create`
- `projection/lower_id.rs`（5）: `LowerIdRecord::container_dev_id`、
  `lower_layer_root_ino`、`real_ino`、`OverlayFs::store_lower_id`、
  `OverlayFs::read_lower_id`
- `copyup/coordination.rs`（1）: `CopyUpTransition`
- `copyup/promote.rs`（3）: `OverlayInode::run_recipe`、
  `OverlayInode::workdir_root_path`、`CommitMarker::commit`
- `copyup/trigger.rs`（1）: `OverlayInode::ensure_upper_authority`
- `dir/whiteout.rs`（2）: `WhiteoutCache`、`WhiteoutCache::new`
- `mount/policy.rs`（2）: `UpperFilesystemCapabilities::can_store_private_xattr`、
  `can_mknod_char`
- `mount/claims.rs`（1）: `OverlayFs::workdir_workspace_path`

### 4.3 编译修正回退为 KEEP（2 条）

- `copyup/promote.rs`: `CommitMarker`
- `projection/lower_id.rs`: `LowerIdRecord`

合计：25 + 85 + 2 = **112**。

## 5. Next Actions

1. **G10 P1 已执行、run07 PASS**；`xattr.rs` 1 行改动 + G10 账本改动待
   user 指令 amend。P2（workdir_workspace_path co-location）未执行，
   报告建议保守 KEEP。
2. G7/G9 仍为规划态，等 user 授权；S-29 deferred 不再派。
3. **安全红线（永久）**：容器 `codex-asterinas-dev` 的 `/root/asterinas` 是
   宿主 `/home/ayd/asterinas` 的 live bind mount；禁止任何
   `rm`/`mv 整目录`/`git reset --hard`/`git clean`/tar 同步；只读检查与
   `docker exec` 编译命令除外。
