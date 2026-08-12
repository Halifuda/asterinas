<!-- SPDX-License-Identifier: MPL-2.0 -->

# B/C-7：Advanced Identity、Export、Data Features

**状态：** 阶段完成（用户确认）；候选能力仍待 Stage D 取舍

## 0. 已确认的范围调整

- `traps`（`P3-08`）排除在当前基础设计和后续实现范围之外；不建立
  trap inode 生命周期，也不依赖它检测底层 layer 拓扑变化。
- `workdir/index cleanup`（`P3-09`）保留为备选的挂载恢复增强；基础路径只要求
  workdir temporary 不进入可见 namespace，不要求本阶段实现完整的崩溃后清理和
  index 验证。
- `fs-verity`（`P3-05`）不纳入当前实现目标；不建立 fs-verity 数据验证层。
- `origin/index`、`workdir/index cleanup` 和 NFS export 均作为可选阶段，不进入
  当前基础实现；若未来推进，顺序固定为 `index → cleanup → NFS export write`。
- NFS export 作为一个整体可选能力，不拆出只读 export 子阶段；若不支持 NFS
  write，则当前初等阶段也不单独支持 NFS read。

**主要对应 meso-components：**

- `origin_index_export`
- `metacopy_verity_data_layers`

其余小部件是对前置模块的扩展，不创建新的 meso-component。

## 1. 总体方向

B/C-7 不是一个新的核心 namespace 模块，而是一组可独立启用的“小部件”。
它们复用 B/C-1 至 B/C-6 已建立的 mount/layer、projection、copy-up、metadata
和 mutation owner；不得各自再建立一套 identity、visibility 或 persistence
authority。每个候选项都先判断是否确实需要纳入基础设计，未纳入时保持基础行为。

## 2. 候选小部件

| 小部件 | 简短语义 | 与前面模块的关系 |
| --- | --- | --- |
| `xino` / UUID / fsid（`P2-01`, `P2-11`） | 将 layer 身份投影为稳定的 `st_ino`、`d_ino` 和 fsid；溢出或不支持时回到明确的降级策略。 | B/C-1 提供 mount/layer identity；B/C-2 负责 identity projection；B/C-3 保持目录 cookie 与编号语义；B/C-4 copy-up 后延续同一逻辑对象身份。 |
| origin / index（`P2-04`, `P3-01`，备选阶段 1） | 用 lower object 的稳定身份关联 upper object，避免 copy-up 或 hardlink 产生错误的第二份 authority，并在 mount/lookup 时验证旧记录。 | B/C-1 提供 `IU`、upper/workdir 生命周期；B/C-2 提供 object identity；B/C-4 提供 copy-up 交接；B/C-5 负责 private xattr；B/C-6 在 whiteout、link、rename 后更新或失效关联。 |
| NFS export（`P3-02`，备选阶段 3） | 将已验证的 identity 编解码为 file handle，并支持 connected/disconnected 对象重建；无 index 或身份不可信时拒绝。它是整体可选能力，不拆出只读 export。 | 依赖 origin/index 和其 cleanup；重建结果回到 B/C-2 的 projection，不绕过 B/C-1 的 mount lifetime，也不改变 B/C-6 的 namespace publication。 |
| `metacopy` / data-only lower（`P3-03`, `P3-04`） | 只先复制 metadata，data 仍来自指定 lower；第一次写入前再完成一次受协调的 data copy-up。data-only layer 只提供 data，不提供 name 或 metadata。 | B/C-1 固定 layer 顺序和能力；B/C-2 保持可见 object identity；B/C-4 管理 data authority、page cache 和写入触发；B/C-5 管理 marker/xattr；B/C-6 只消费最终 publication。 |
| fs-verity（`P3-05`，排除） | 不纳入当前实现目标；不建立 deferred data 的 fs-verity digest 验证。 | 不改变 B/C-4 的基础 data-authority transition；后续若重新纳入，需单独恢复其 source 验证和 private metadata contract。 |
| `redirect_dir`（`P2-02`） | 为 lower/merged directory 的跨目录 rename 提供可解释的 redirect；未启用时继续返回默认 `EXDEV`。 | B/C-6 决定 rename recipe 和 publication；B/C-4 提供 directory promotion；B/C-5 写入/校验 redirect xattr；B/C-2 在后续 lookup 中解释 redirect。 |
| workdir/index cleanup（`P3-09`，备选阶段 2） | 在新 mount 时清理上次失败留下的 workdir temporary，并可选验证或清理 stale index。 | 属于 B/C-1 的 mount/teardown hygiene；基础路径只保证 temporary 不进入可见 namespace，也不引入 `P3-08` trap inode。 |

## 3. 共同约束

- 小部件的配置和能力检查在 mount 阶段完成；运行时只消费已发布的 immutable
  policy 和 pinned layer/object references。
- 新增记录必须有单一 owner、明确发布点和失败后的 stale/cleanup 结果；不以
  路径、缓存名称或未经验证的 file handle 替代稳定 identity。
- 先进功能不得改变 B/C-1 至 B/C-6 的基础默认语义：没有明确 policy 时，
  lower/merged directory 的跨目录 rename 仍为 `EXDEV`，whiteout/opaque 仍是
  visibility barrier 而不是用户可见对象。
- 锁继续遵守 `DIR -> CUL -> INODE -> WL -> UPPER`；本模块不引入新的锁层，
  blocking underlying call 或可重入 callback 后必须重新验证 source、policy 和
  publication state。

## 4. 阶段结论

- B/C-7 的讨论目标已完成：它定义了高级 identity、export、data 小部件与
  B/C-1 至 B/C-6 的依赖边界，不新增核心 namespace owner。
- 当前基础实现排除 `traps` 和 `fs-verity`；`index`、`workdir/index cleanup`、
  NFS export 保留为有序备选阶段。
- Stage D 只需决定是否开启这条备选链，不得把 NFS export 拆成独立的只读阶段。
