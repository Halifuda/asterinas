<!-- SPDX-License-Identifier: MPL-2.0 -->

# Handoff: Overlayfs 代码结构实际搬移（2026-08-23）

**Status:** PHASE 2 COMPLETE — B/C/A/T/T7/B5 已执行，`cargo check` / `cargo clippy` 均通过。

## Goal

把当前 active overlayfs 代码按 `structure-design-proposal.md` 的目标结构，从“当前平铺/旧布局”搬移到新的 `fs/` + `inode/` + `real.rs`/`layer.rs` 布局。

这次不是重新实现，而是“搬移 + 定点语义修改”。最终新树要能独立编译、通过现有测试，然后删除 `old/` 参考区。

## 当前事实

- 当前 active overlayfs 代码在：
  `kernel/core/src/fs/fs_impls/overlayfs/`
- `legacy_fs.rs` 是旧内容，不参与本次搬移，保留为参考。
- 设计文档已经更新：
  - `structure-design-proposal.md`：目标结构 + 章节顺序已改为 top-down。
  - `structure-design-appendices.md`：包含 A/B/C 逻辑修改点、锁序附录 E。
  - `symbol-file-map.md`：符号→目标文件映射 + 文件级迁移表。
- Phase 0 开始时 `git status -- kernel/core/src/fs/fs_impls/overlayfs` 为 clean；handoff 原先提到的 `mount/layers.rs`、`mount/options.rs`、`superblock.rs`、`workdir.rs` 未提交修改在当前工作区不存在，搬移基线即当前 HEAD。
- 环境：`codex-asterinas-dev` container 已打开，rust-analyzer / jq 可用；当前 rust-analyzer nightly 的 LSIF 会 panic，已改用 VSCode 内置 standalone rust-analyzer 0.3.3016 生成成功。

## Phase 0 执行记录（2026-08-23）

- [x] 生成 LSIF：`/tmp/asterinas.lsif`（容器内 162MB，projectRoot `/root/asterinas`）。
  - 使用 `/usr/local/bin/rust-analyzer-vscode`（0.3.3016-standalone）生成；容器默认 nightly rust-analyzer 1.99.0 的 `lsif` 会 panic，不能使用。
- [x] 创建 old/ 参考区：`kernel/core/src/fs/fs_impls/overlayfs/.agents/refactor/old/`
  - 已将全部 active/legacy 代码（33 个跟踪文件，含 `legacy_fs.rs`）从主树移入 old/，并用 `git show HEAD:...` 逐一比对，全部与 HEAD 一致。
- [x] 清空主代码空间：`kernel/core/src/fs/fs_impls/overlayfs/` 现在除 `.agents/` 外没有 `.rs` 文件或旧模块目录。
- [x] 按用户要求不 ignore old/：`.agents/refactor/` 保持可跟踪，old/ 代码将随本次 commit 纳入 git。
- [x] **LSIF 提醒**：`/tmp/asterinas.lsif` 是在搬移前基于旧代码生成的，索引中的 document URI 仍指向原主树路径（如 `file:///root/asterinas/kernel/core/src/fs/fs_impls/overlayfs/...`）。这些旧路径现在已不存在，实际代码已移到 `.agents/refactor/old/` 下。使用 LSIF 做符号查询时要注意路径映射，或在新树成形后重新生成。
- 当前 `git status` 对该 overlayfs 路径显示为旧文件删除 + old/ 新文件待跟踪 + handoff 修改；将作为一个 commit 提交。

## 搬移大原则

1. **old/ 只是参考区，不参与编译**
   - 把当前 active 代码复制到不参与编译的参考区，例如：
     `kernel/core/src/fs/fs_impls/overlayfs/.agents/refactor/old/`
   - 新代码直接落在真实代码树里，最终只保留新树。
   - 新树测试通过后再删除 `old/`。

2. **先结构搬移，后语义修改**
   - 第一轮只做“搬移/拆分/合并/改名”，尽量保持函数体不变。
   - 第二轮才处理 proposal 附录里的 A/B/C 逻辑修改点。
   - 不要把语义修改混进搬移 commit，否则无法定位回归。

3. **能直接搬就直接搬，不重写**
   - 因为 `old/` 保留旧代码快照，实际搬移是**复制**：从 `old/` 复制到新路径，再删除真实树中的旧文件。
   - 操作上不要用 `git mv` 直接移动 `old/` 里的文件；`old/` 始终保留原始参考。
   - 只有 proposal 明确要求删除/合并/内部符号移动时才改结构。

4. **注释策略**
   - 文件级 top doc comment 基本重写。
   - 解释“为什么”、不变量、锁序、Linux 依据的注释跟着代码走，不丢。

5. **禁止 basename 操作**
   - 所有 subagent 任务、脚本、指令必须写完整路径。
   - 新旧同名文件很多，例如 `inode.rs` vs `inode/mod.rs`、`permission.rs` 等。
   - 以 `symbol-file-map.md` 的文件级迁移表为唯一搬移依据。

6. **用编译器当 checklist，用 `rg` 当确认手段**
   - 每完成一个模块切片就 `cargo check`（除非刻意采用“最后统一编译”的批次策略）。
   - 删除旧符号前用 `rg` 确认没有残留引用。
   - 不要为了安心通读整个文件。

## 可顺手移动/删除的结构

### 可顺手移动（纯搬移）

- 文件级迁移表中标为“直接搬”的文件：
  - `fs_type.rs` → `overlayfs/fs_type.rs`
  - `superblock.rs` → `overlayfs/fs/mod.rs`
  - `mount/options.rs` → `overlayfs/fs/mount/options.rs`
  - `mount/claims.rs` → `overlayfs/fs/mount/inuse.rs`
  - `projection/identity.rs` → `overlayfs/inode/identity.rs`
  - `readdir_index.rs` → `overlayfs/inode/readdir.rs`
  - `workdir.rs` → `overlayfs/inode/copyup/workdir.rs`
  - `dir/*` → `overlayfs/inode/dir/*`
  - `metadata_security/{permission,metadata,xattr}.rs` → `overlayfs/inode/{permission,metadata,xattr}.rs`
- 这些只改路径、`use`、可见性和文件顶部 doc comment，不改函数体。

### 可顺手删除（机械、无行为变化）

- `*_impl` 转发壳：
  - `inode.rs` 里的 `read_at_impl` / `write_at_impl` / `open_impl` / `resize_impl` / `fallocate_impl` 等；
  - `dir/mod.rs` 里的 `create_impl` / `mknod_impl` / `link_impl` / `unlink_impl` / `rmdir_impl` / `rename_impl`；
  - `metadata_security/*` 里的 `set_*_impl` / `get_xattr_impl` / `set_xattr_impl` 等。
  当 trait 方法直接落到新文件后，这些转发壳可以删除（T7）。

- 模块 glue / re-export：
  - `mount/mod.rs`
  - `projection/mod.rs`
  - `metadata_security/mod.rs`
  子模块搬完后可删除或缩成纯 `mod` 声明。

- 局部 carrier 类型（内联/参数化）：
  - `PreparedTemp`
  - `PromoteTarget`
  - `CommitMarker`（改为局部 bool）
  - `WorkdirWorkspace`（并入 `UpperWorkdirInuse.workspace`）
  - `LookupOutcome`（改为 `Lookup` 或元组）
  - `XattrPolicy`（ZST，改为自由函数/关联函数）

这些删除大多是附录 T5 的“类型消灭”机械改写，不引入新行为；建议随对应文件搬移一起做，不要单独留到 Phase 2。

### 可顺手做但需小心的结构简化

- `RealPath.inode` 字段删除：proposal 已确定为机械简化，但需要同步修改所有调用点；建议在创建 `real.rs` 时一起做，并单独确认没有行为变化。
- `OverlayInode` 双构造点合一：属于构造路径整理，建议在 `inode/mod.rs` 成形时处理，但需要额外注意初始化顺序。

### 不可顺手删除（必须 Phase 2 单独处理）

- `BindingCache` / `PositiveBinding` / `HiddenEvidence` / `BindingKey`：与 A4/A5 绑定缓存消除绑定，不能只当结构删除。
- `revalidate_absent_impl` / `REVALIDATE_ABSENT`：A2 语义变化。
- `stale_upper` 相关（`translate_stale_upper_enoent`、`is_stale_upper`）：A1 语义保留/换载体。
- facts → `Once<RealObject>`、锁合并、调用顺序调整：B1–B4、C1–C4 需要单独做。


## 省 Token 原则

- 不重新解释设计：先读 proposal / appendices / symbol-file-map，不在 handoff 里重复大段设计。
- 不整文件搬运到上下文：只针对当前要移动的符号/文件做局部读取。
- 用 LSIF 查跨文件引用；小范围残留用 `rg` 确认，不用肉眼扫全树。
- subagent 任务切成窄切片，每个任务只给：
  - 旧路径（old/ 内）
  - 新路径
  - 允许的改动类型
  - 验证命令
- 旧代码参考放在 `old/`，避免每次 `git show HEAD:...` 消耗上下文。
- 批量机械操作尽量脚本化（从迁移表生成 `cp` / `mkdir` / 删除真实树旧文件的命令）。
- 最终提交时可用 `git diff -M` 查看 rename 效果，但操作本身是 copy + delete。
- 语义修改点一个一个来，每个点单独验证、单独 commit。

## 具体执行流程

### Phase 0：准备

1. 确认工作区未提交修改是否纳入搬移基线。
   - 若纳入：先把这些修改整理成独立 commit，或明确搬移时从当前工作区内容开始。
   - 若不纳入：从当前 HEAD 开始，暂存/忽略这些文件。
2. 创建 old/ 参考区：
   ```bash
   mkdir -p kernel/core/src/fs/fs_impls/overlayfs/.agents/refactor/old
   cd kernel/core/src/fs/fs_impls/overlayfs
   tar --exclude='./.agents' -cf - . | tar -xf - -C .agents/refactor/old
   cd - >/dev/null
   ```
   这样会排除 `.agents/` 自身，避免递归复制。
3. 阅读三份设计文档：
   - `structure-design-proposal.md`
   - `structure-design-appendices.md`
   - `symbol-file-map.md`（重点是文件级迁移表）
4. 生成一次 rust-analyzer LSIF 索引，用于搬移前的跨文件引用分析：
   ```bash
   cd /root/asterinas && rust-analyzer lsif . > /tmp/asterinas.lsif 2>/tmp/lsif.log
   ```
   后续每个大切片完成后按需重新生成；小改动不必每次重建。

### Phase 1：纯结构搬移

按 `symbol-file-map.md` 的文件级迁移表执行：

1. 从 old/ 中读取源文件。
2. 在目标路径创建新文件，**复制**代码（不要移动 old/ 中的文件）。
3. 只修改：
   - `use` / `mod` 路径
   - `pub(in ...)` / 可见性
   - 文件顶部 doc comment
   - 必要的模块声明
4. 每完成一个可编译切片后运行：
   ```bash
   cargo check -p asterinas --target x86_64-unknown-none
   ```
   如果采用“最后统一编译”策略，则至少每完成一个目录/模块做一次局部检查或 `rg` 确认。
5. 真实树中的旧文件随搬随清，不留两份真代码；`old/` 保留不动。
   - 可以临时用 re-export shim 过渡，但不要长期保留。
6. 最终提交时用 `git diff -M` 确认 git 能识别为 rename；但操作本身是 copy + delete。

### Phase 2：语义修改点（规划已确认，2026-08-23，暂未执行）

按 `structure-design-appendices.md` 的 A/B/C/T 列表执行，但有以下已确认约束：

- **A1**：不依赖 #3745。使用设计里已有的“Once 有内容 = 曾经 upper-backed；fresh 扫描 absent 且无 whiteout → ESTALE”方案，目标 inode 在 remove 路径直接可得，不等待 resolved-inode PR。
- **B5**：整个 Phase 2 最后再决定（`ensure_readdir_index` 复核 + `EIO` 分支删留）。
- **C3**：按模型修改即可，不需要额外死锁担忧；执行时仍做一次“持 lock 不取 CUL”的入口审计。
- **T7**：顶层模块 `fs` / `inode` / `real` / `layer` 内，`pub(in overlayfs)` 与 `pub(super)` 是同一概念；统一优先用 `pub(super)`。
- **Creator 权限**：允许 Creator 运行 `cargo check` 和 `cargo clippy` 用于自身验证。
- **xfstests**：Phase 2 当前全部不做，只做编译/静态验证。
- **commit message**：不要带 A/B/C/T 字母标号；用描述性短语，例如 `overlayfs: remove binding cache from lookup path`。

执行顺序（每个点单独 commit）：

1. **状态模型 B1/B2/B3/B4/B6**：`facts → Once<RealObject>`、删除 `key`、两锁合并、`append_write` 改持新锁、引入 `spin::Once`。
   - **同时处理 `kind` / `PositiveKind`**：旧 `ObjectFacts.kind` 存储字段随 facts 替换删除；`PositiveKind` 不是目标结构符号，按 proposal 修正后不再保留，改用 `RealObjectStack::is_merged()` 等派生判断。
   - **`copyup_transition` 对齐 proposal**：字段从 `Mutex<Option<CopyUpTransition>>` 改为 `Mutex<CopyUpTransition>`（非 Option），并调整 `try_record_copyup_transition` / 调用点。
2. **调用顺序 C1/C2/C3/C4**：先升格后取锁、源升格前置、commit 段入 `publication_parent.lock`、`object_id` 惰性化（仍保存在 `OverlayInode.object_id`，只是计算时机变惰性）。
3. **行为语义 A1/A2/A3/A4/A5**：删除 BindingCache 及发布/失效逻辑，stale 用 Once 承载，负 dentry 永远信任，commit 活性复查。
   - **必须落地 `Lookup` / `NegativeLookup`**：当前 `inode/lookup.rs` 仍用 `LayerLookup` / `LookupOutcome` / `Binding` 临时类型；按 proposal 正源，最终 lookup 返回形态应为 `Lookup` / `NegativeLookup`，在 A4/T5 时恢复。
4. **类型消灭/去重 T5/T6**：
   - T5：`WorkdirWorkspace`、`LookupOutcome`、`PreparedTemp`、`PromoteTarget`、`CommitMarker`、`XattrPolicy`、`PositiveBinding`/`HiddenEvidence`/`BindingKey` 等按提案消灭；`OverlayInode` 双构造点合一。
   - T6：暂存发布骨架统一为 token 化 `publish_temp`；属性迁移两套合一；`mknod_object_type` 单点化。
   - T6 已采纳的补充项也要做：best-effort impure marker 刷新 helper；whiteout 发布后的索引收尾 helper（按 remove/rename 共同子集抽，保留各自差异）；`RealObjectStack` 字面量构造统一走构造器；workdir root 单一权威来源。
5. **可见性收窄 T7**：范围最大、最机械，最后做；顶层模块用 `pub(super)` 代替 `pub(in overlayfs)`。
   - **同时按 proposal 文件结构把 `AccessType` 从 `overlayfs/mod.rs` 移到 `inode/permission.rs`**（proposal 正源；symbol-file-map 已同步修正）。
6. **B5 裁决**：已决断并执行——删除 `ensure_readdir_index` 的复核 + `EIO` 分支（只读验证确认目录升格保持可见序列不变）。
7. **最终验证**：`cargo check` + `cargo clippy`（xfstests 不做）。

> 遗漏检查补充（2026-08-23 通读三文件后）：
> - `PositiveKind`：proposal 文件结构原书写有误；设计描述中的 `OverlayInode` 结构并没有该字段。已修正 proposal 文件结构和 symbol-file-map，Phase 2 不保留 `PositiveKind`。
> - `Lookup` / `NegativeLookup`：当前 `inode/lookup.rs` 缺少（proposal 明确列出；现被 Phase 2 将消灭的 Binding 临时类型替代），在 A4/T5 时恢复。
> - `AccessType`：当前仍在 `overlayfs/mod.rs`，但 proposal 文件结构要求它在 `inode/permission.rs`；列入 T7 一并移动。
> - `copyup_transition`：当前是 `Mutex<Option<CopyUpTransition>>`，proposal 要求 `Mutex<CopyUpTransition>`；列入 B 步骤对齐。
> - 其余目标文件与 proposal / symbol-file-map 基本一致。

### Phase 3：清理与验证（已更新 2026-08-23）

1. **新代码与 proposal 对照**：要求没有冗余代码、没有错位摆放的代码块、所有可见性均收窄到它需要的最窄。
2. **格式与警告**：运行 `rustfmt`；检查 `cargo check` / `cargo clippy` 的所有 warning，逐一决策并解决。
3. **测试**：运行相关测试/验证（具体范围待用户确认）。

#### Phase 3 Step 1 Workflow 编排（已记录，暂不执行）

- **并行 Creator × 3**（write-set 互斥）：
  - `p3s1-top-fs`：顶层 `mod.rs` / `fs_type.rs` / `real.rs` / `layer.rs` + `fs/` 子树
  - `p3s1-inode-core`：`inode/mod.rs` / `inode_cache.rs` / `lookup.rs` / `identity.rs` / `readdir.rs` / `data.rs` / `permission.rs` / `metadata.rs` / `xattr.rs`
  - `p3s1-inode-leaf`：`inode/copyup/` + `inode/dir/`
- 每个 Creator：
  - 对照 proposal / symbol-file-map / appendices 审计自己子树；
  - 删除冗余代码（重复 helper、无用 re-export、死字段/类型）；
  - 修正错位摆放（按 proposal 文件归属；跨子树移动只报告不代做）；
  - 收窄可见性到最窄（顶层模块统一用 `pub(super)`，不滥用 `pub(in overlayfs)`）；
  - 可运行 `cargo check` / `clippy` 做局部验证；
  - 不改 `old/`、`.agents` 记录、`legacy_fs.rs`。
- **收尾 Checker × 1**：跑一次 `cargo check` 确认可编译；用 `rg` 检查旧路径/旧符号残留。
- **主代理复核**：检查 git diff 与 scope，输出汇总。

## 参考资料

- 目标结构 / 叙事：
  `kernel/core/src/fs/fs_impls/overlayfs/.agents/designdoc/structure-design-proposal.md`
- 逻辑修改点 / 锁序讨论：
  `kernel/core/src/fs/fs_impls/overlayfs/.agents/designdoc/structure-design-appendices.md`
- 符号归属 + 文件级迁移表：
  `kernel/core/src/fs/fs_impls/overlayfs/.agents/designdoc/symbol-file-map.md`
- 当前代码：
  `kernel/core/src/fs/fs_impls/overlayfs/`
- 旧参考区（Phase 0 后创建）：
  `kernel/core/src/fs/fs_impls/overlayfs/.agents/refactor/old/`
- Linux 参考：
  `~/linux/fs/overlayfs/`（尤其 `copy_up.c`、`dir.c`、`inode.c`、`util.c`）
- 工作区规范：
  `AGENTS.md`、`CLAUDE.md`

## Next Action（给下一个 agent）

1. Phase 0/1/2 均已完成并已提交；Phase 2 的 B/C/A/T/T7/B5 已执行，`cargo check` / `cargo clippy` 通过。
2. 下一步是 Phase 3（按用户更新后的定义）：proposal 对照与冗余/错位/可见性收窄检查 → `rustfmt` + warning 逐项决策解决 → 测试。
