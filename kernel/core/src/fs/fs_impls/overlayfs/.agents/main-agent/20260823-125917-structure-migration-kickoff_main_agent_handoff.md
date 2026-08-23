<!-- SPDX-License-Identifier: MPL-2.0 -->

# Handoff: Overlayfs 代码结构实际搬移（2026-08-23）

**Status:** PHASE 0 COMPLETE — LSIF 已生成，旧代码已移入 `old/`，主代码空间已清空，等待 Phase 1 结构搬移。

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

### Phase 2：语义修改点

按 `structure-design-appendices.md` 的 A/B/C/T 列表逐个执行：

- A1–A5：行为语义变化（stale、negative dentry、commit 活性复查等）。
- B1–B6：状态模型重写（facts → Once、锁合并等）。
- C1–C4：调用顺序/协议重排（先升格后取锁等）。
- T5–T7：类型消灭、去重、可见性收窄。

每个点建议：

1. 只改一个点。
2. 运行对应测试或 xfstests。
3. 保留 proposal 提到的可回撤开关，先对比新旧行为。
4. commit message 标注编号，例如 `A3: commit段活性复查`。

### Phase 3：清理与验证

1. 新树完整 `cargo check`、`make check`、相关 overlayfs 测试。
2. `rg` 确认没有旧路径残留引用。
3. 确认 `legacy_fs.rs` 和 `old/` 不影响编译。
4. 测试通过后删除 `old/` 参考区。
5. 最后整理 commit 历史，确保每个 commit 可读。

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

1. Phase 0 已完成（LSIF、old/、清空主树），无需重复。
2. 从 `symbol-file-map.md` 文件级迁移表选第一个可独立完成的切片开始 Phase 1，例如：
   - 先建 `mod.rs`、`fs_type.rs`、`layer.rs`、`real.rs` 与 `fs/`、`inode/` 骨架；
   - 或按依赖从 `real.rs`/`layer.rs` 基础类型开始。
3. 每个切片完成后运行 `cargo check`（或按既定批次策略验证），并在 handoff/commit 中记录进度。
4. 不要同时开始 Phase 2 的语义修改；先把 Phase 1 的结构搬移推进到可编译状态。
