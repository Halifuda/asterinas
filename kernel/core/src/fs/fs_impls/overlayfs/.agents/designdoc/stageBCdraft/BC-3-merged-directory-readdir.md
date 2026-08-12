<!-- SPDX-License-Identifier: MPL-2.0 -->

# B/C-3：Merged Directory、Readdir

**状态：** 设计完成，已阶段签收（Stage B/C 最终收口）

**对应 meso-component：**

- `merged_readdir_cache`

**覆盖 Micro-feature：**

- `P0-13`：non-merged directory 的 underlying delegation；
- `P0-14`：merged directory 的合并、去重和 readdir；
- `P1-31`：目录结构变化后的 readdir index 维护；
- `P2-03`：impure-directory marker 与对应 overlay xattr。

**前置：** B/C-1 已发布的 layer/mount lifetime，以及 B/C-2 已定义的
directory projection、visibility barrier、identity carrier、BindingCache 和
父目录 `DIR` 一致性域。

## 25. 设计结论

B/C-3 负责把一个 Overlay 目录投影成可供 VFS `readdir` 消费的目录序列。
它同时覆盖 upper-only directory 和需要合并 upper/lower 的 directory。

本稿采用以下基本模型：

1. Overlay directory 的目录级状态拥有一个当前的、可变的
   **ReaddirIndex**。该 index 是 `merged_readdir_cache` meso-component 的
   核心；“merged cache”是历史名称，不表示只有 merged directory 才能使用
   它。
2. 所有对外目录项都使用 Overlay 自己的 cookie namespace。upper-only
   路径可以把目录读取委托给 underlying，但不能把 underlying cookie 原样
   暴露给 VFS。
3. 打开的 FD 不保存 Overlay 私有的 cache、snapshot、version 或 cursor。
   FD 只由 VFS 保存通常的文件 offset；每次 `readdir` 都把这个 offset 解释
   为当前 Overlay directory 的 cookie。
4. readdir 和 namespace mutation 在同一个 Overlay parent `DIR` 一致性域
   内执行。mutation 成功后直接维护当前 index；不能依赖“下一次 readdir
   重新发现 mutation”作为正常一致性机制。
5. whiteout、opaque 和其它 visibility barrier 是投影输入，不是可见目录项。
   可见目录项进入 ReaddirIndex；隐藏状态由 B/C-2 BindingCache 和目录级
   barrier state 表示。

因此，本阶段不维护每个 FD 的旧目录视图，也不保存多个历史 cache。已有 FD
在下一次 `readdir` 时看到的是当前 index；它的旧 offset 通过稳定 cookie 继续
解析。

## 26. 目录级状态与所有权

“OverlayDirectoryState”表示目录级责任角色，不冻结一个新的 Rust 结构名。
它附着于 Overlay directory inode/carrier，至少负责以下状态：

- upper/lower directory 的 pinned 引用和 layer 顺序；
- B/C-2 提供的 projection、barrier 和目录 identity 事实；
- 当前 ReaddirIndex 及其有效/需重建状态；
- 对该目录的 cookie 分配和不复用规则；
- 对当前 index 的发布、增量修改和失效权限；
- 必要时的 impure/origin identity 辅助事实。

OverlayDirectoryState 不拥有 underlying filesystem 的持久化 namespace。真实
目录项、whiteout entry、whiteout xattr、opaque xattr 和其它 upper metadata
仍由 underlying filesystem 保存。

### 26.1 ReaddirIndex

ReaddirIndex 是一个目录级、单一当前实例的可遍历 index。它不是 FD 私有
cache，也不是第二份 namespace authority。它保存用户可见目录序列所需的
投影结果，包括：

- 可见名称；
- Overlay inode identity 和 `InodeType`；
- 稳定的 Overlay continuation cookie；
- 当前 entry 的可见顺序和后继关系；
- 必要时对 B/C-2 positive BindingCache entry 的引用。

entry 不需要保存 underlying raw dirent，也不需要为普通 readdir 保存
underlying cookie。只有在未来选择 lazy source scan、source revalidation 或
调试 provenance 时，才可以附带 layer 与 underlying position。

ReaddirIndex 可以使用数组、slot table、ordered map 或其它结构实现，但其
语义必须同时满足 cookie 查找和按可见顺序遍历。单独使用“当前 Vec 下标”不
能承受任意 mutation，因为删除和插入会改变下标。

### 26.2 BindingCache 的关系

B/C-2 的 BindingCache 是 Overlay 实例级的 `(parent identity, name)` 解析
缓存。它可以保存：

- positive binding：该名字当前绑定到哪个 Overlay object；
- hidden/negative binding：该名字被 whiteout、opaque barrier 或其它可证明
  的 visibility 事实隐藏；
- unknown 或需要 revalidation 的证据状态。

ReaddirIndex 只保存 positive、可见的投影结果。隐藏 binding 不作为目录项
输出，也不占用可见 cookie。Parent directory state 可以引用或查询全局
BindingCache，但不应再复制一份完整的 per-parent binding/whiteout authority。

物理 whiteout 仍然存在于 upper directory；BindingCache 中的 hidden state
只是 Overlay 对该物理事实的解析结果，并且可以在失效后重新从 underlying
观察得到。

## 27. Overlay cookie 语义

`readdir_at` 的输入 offset 和 `DirentVisitor::visit` 的 continuation offset
都使用 Overlay cookie。underlying layer 的 cookie 是内部数据，不直接作为
merged 或 upper-only 路径的对外 cursor。

cookie 需要满足：

- 在一次输出序列中单调递增；
- 已经分配并可能暴露给用户的 cookie 不重新分配给其它逻辑位置；
- mutation 后仍能用旧 cookie 在当前 index 中找到合理的继续位置；
- `.`、`..` 和普通 entry 使用同一 Overlay cookie namespace；
- `seekdir(0)` 只把 VFS offset 置零，不创建 FD 私有 cache。

当前 Asterinas `InodeHandle` 会把 `readdir_at` 返回的 `usize` 加到自身
offset。因此实现必须保持：

```text
returned_usize = next_overlay_cookie - input_overlay_cookie
```

目录项输出的 `d_off` 是该目录项之后的 Overlay continuation cookie，而不是
underlying cookie，也不是写入用户 buffer 的字节数。

### 27.1 删除、隐藏和重新出现

删除可见 entry 不要求重建整个 index：

- 如果 index 可以按 cookie 做 `first cookie > offset` 查询，删除 entry 后
  可以直接移除节点；旧 offset 会自然落到下一个现存 cookie；
- 如果实现使用显式 successor 指针，则可以保留 tombstone 或 successor
  redirect，使旧 cookie 继续可遍历；
- 删除后的 cookie 不重新分配给后来新出现的 entry。

whiteout 隐藏一个当前可见的 lower entry 时，移除或 tombstone 该可见 entry，
但不把 whiteout 自身放入可见 index。若 upper winner 被删除、lower 同名
entry 重新成为可见 winner，可以保留同一个逻辑名字的 cookie，只更新其
binding 和 identity；若该名字此前完全不可见，则分配新 cookie。

新 entry 的插入必须维护遍历顺序和旧 cookie 的稳定性。实现可以使用有序
cookie、order-maintenance label 或其它等价机制；不能因为普通插入而重编号
已经暴露的旧 entry。若一次 mutation 无法在不破坏 cookie 不变量的情况下
完成局部更新，则将 index 标记为 `NeedsRebuild`，而不是静默重编号。

## 28. 可见序列与 source projection

### 28.1 特殊项

`.` 和 `..` 是 Overlay 目录的可见特殊项，各出现一次：

- `.` 使用当前 Overlay directory 的 identity；
- `..` 使用 Overlay parent 关系提供的 identity；
- 它们不参与 upper/lower 普通名称竞争；
- 它们的 continuation cookie 仍属于 Overlay cookie namespace。

### 28.2 Merged directory

当 B/C-2 判定目录需要 upper/lower projection 时，ReaddirIndex 按以下规则
维护可见序列：

1. upper directory 的可见普通项优先，并保持该 underlying directory 的
   枚举顺序；
2. 如果没有 `opaque=y` barrier，再按 mount 规定的 top-to-bottom 顺序观察
   lower directories；
3. 对同名项使用 first-visible-wins；去重 key 是可见名称，不是 inode number；
4. upper whiteout 隐藏同名 lower 项，whiteout 不进入可见 index；
5. `opaque=y` 阻止其后的 lower directory 继续产生可见项；
6. `opaque=x` 仍按 B/C-2 的 xwhiteout policy 解释，不把 marker 本身当作
   可见目录项；
7. private name、无法证明可见性的 entry 和无法可靠解析的 marker 不得被
   乐观放入可见 index。

BindingCache 提供名称的 positive/hidden projection，ID carrier 提供对外
identity。ReaddirIndex 只负责把这些结果组织成稳定的输出序列。

### 28.3 Upper-only directory

upper-only 目录仍使用同一个 Overlay cookie namespace 和 ReaddirIndex。其
source 读取可以直接委托 upper underlying directory，保留 underlying 的
I/O、entry name/type 和错误行为，但输出必须通过 Overlay boundary：

- 把 underlying entry 映射到 Overlay identity；
- 把 underlying continuation position 转换为 Overlay cookie，或从当前
  ReaddirIndex 取得 Overlay cookie；
- 处理 `.`、`..`、private name 和可能的 whiteout；
- 不把 raw underlying `d_ino` 或 cookie 当作无条件的对外值。

因此 “direct” 只表示 source read delegation，不表示绕过 Overlay 的
cookie/index/identity 投影。

## 29. Readdir 与 index 建立/维护

一次 readdir transaction 遵循以下顺序：

```text
acquire Overlay parent DIR
    -> inspect current ReaddirIndex state
    -> use current index, or observe sources and build/reconcile it
    -> update BindingCache and identity projection as required
    -> publish the current index state
    -> iterate visible entries and advance the VFS offset
release Overlay parent DIR
```

如果当前 index 有效，readdir 只从 index 中查找 offset 后的可见 entry；不必
再次访问 underlying cookie。若 index 尚未建立或被标为 `NeedsRebuild`，则在
同一个 `DIR` transaction 内按 B/C-2 的 layer、barrier 和 binding 规则扫描
source，形成一个完整的可见序列，然后替换当前 index。

不能发布只完成 upper 或只完成部分 lower 的可见 index。source read、whiteout
解析、identity projection 或内存分配失败时，保留 `NeedsRebuild` 状态并按
当前操作边界返回错误或重试；不能把 partial result 当作正常 cache。

### 29.1 Mutation 的增量维护

成功的 create、mkdir、unlink、rmdir、link、rename、copy-up、whiteout 或
opaque 变化，在 underlying namespace commit 后，必须在同一个 parent `DIR`
一致性域内维护受影响目录的 BindingCache、barrier state 和 ReaddirIndex。

典型更新包括：

- 新建可见 upper entry：插入 positive binding、identity 和新 cookie；
- unlink upper winner：若存在 lower fallback，则更新原逻辑 entry 的 binding；
  否则移除可见 entry；
- 创建 whiteout：写入 hidden binding，移除被遮蔽的可见 lower entry；
- 删除 whiteout：重新解析该名字，必要时把 lower entry 插入 index；
- rename：同时维护 source parent 和 target parent 的 index，目标覆盖和
  whiteout 也纳入受影响集合；
- opaque 变化：更新目录级 barrier；若影响范围无法局部确定，标记
  `NeedsRebuild`；
- copy-up 或 origin 变化：更新 binding/identity；impure 只作为 identity
  校正提示，不改变 cookie namespace。

mutation 不能只修改 underlying 而把 index 更新留给下一次 readdir 猜测。
如果 namespace 已经提交但增量 index 更新失败，则保留已提交的 namespace，
把 index 标为 `NeedsRebuild`，并通过规定的错误边界报告该状态；不能发布一个
看起来有效但内容不完整的 index。

## 30. 有效性与 stale state

本设计不使用 per-FD version，也不保存多个历史 ReaddirIndex。当前 index
只有两类基本状态：

- **Valid：** 可以按当前 cookie 规则为任意 FD 提供 readdir；
- **NeedsRebuild：** 不能证明当前 index 覆盖最新的 namespace/projection，
  下一次 readdir 必须在 `DIR` transaction 内重建或返回错误。

在所有 Overlay-visible mutation 都经过同一个 `DIR` consistency domain、并且
mutation 在提交后立即维护 index 的前提下，不需要额外 generation 来判断
FD 是否过时。FD 没有 Overlay private state；它的 offset 始终针对当前
directory index 解释。

如果 underlying filesystem 允许绕过 Overlay 直接发生 namespace 变化，或者
存在不能通过现有 mutation event 观察到的变化，则必须由 B/C-2 的
revalidation policy 提供额外检测。这属于 external-change detection，不是
本阶段的 per-FD snapshot 机制。

## 31. Impure 与 identity projection

Linux 的 `trusted.overlay.impure` xattr 表示 upper directory 可能包含带有
lower origin 的 non-pure upper entry。B/C-3 将其定义为 identity/origin 侧
的辅助事实：

- 它不决定目录是否需要 upper/lower 名称合并；
- 它不创建或分配 cookie；
- 它不把任何 entry 加入可见 index；
- 它可以使 readdir 对 upper entry 做额外 lookup/stat，以修正对外
  `d_ino` 或其它 Overlay identity 投影；
- 它可以作为持久化提示，帮助决定是否需要 identity revalidation。

如果 B/C-2 已经为所有 positive binding 始终提供正确的 Overlay identity，
impure 在基础 readdir 正确性上不是必需的，可以作为兼容 Linux xattr 的
advisory marker 和 fast-path 优化保留。marker 未知或读取失败时，不得因此
暴露未经投影的 raw identity；应继续使用保守的 Overlay identity path。

## 32. Lock、BIO 与 callback 边界

readdir 和 mutation 都遵循“一次 Overlay parent `DIR` transaction”规则：

```text
DIR
  -> read/update projection and index state
  -> underlying directory reads or writes
  -> BindingCache/identity/index publication
  -> output or commit result
```

同一个逻辑操作不能释放 `DIR` 后再重新获取它来完成剩余步骤。普通
underlying directory I/O、xattr I/O 和必要的 BIO 可以在该 sleep-capable
一致性域内发生。

`INODE`、`UPPER` 等其它锁只能按全局规定的顺序短暂取得；不得用 spin-based
锁包住可能 sleep 的 underlying 操作，也不得让 visitor callback 触发未规定
的递归 `DIR` acquisition。涉及多个 parent 的 rename 必须按照统一顺序取得
相应的 directory consistency domains。

## 33. 错误、生命周期与回收

- OverlayDirectoryState 持有 readdir 所需的 layer/directory 引用，直到
  index 和 source operation 不再使用它们；不能让 index 中的 source/binding
  引用悬空。
- BindingCache entry 可以被 index 引用，但其内容不能在旧引用仍可见时原地
  改写成不相容的对象；source/identity 变化应通过受保护的替换或明确的
  revalidation 完成。
- 可见 entry 删除后可以采用数值 cookie 查找让旧 offset 自然跳过已删除
  entry；如果采用 successor/tombstone 节点，必须保证旧 cookie 的解析关系
  不会悬空，且不能无界累积没有用途的 tombstone。
- cookie 永不复用；entry 重新出现时分配新的 cookie，除非它只是同一逻辑
  名称的 source/binding 重新绑定。
- visitor 因 buffer 不足或其它原因停止输出时，readdir 返回已经消费的
  Overlay cursor 增量；不会把输出字节数混入 cursor 语义。
- 任何失败都不得发布 partial index。已提交的 underlying mutation 不因
  index publication 失败而回滚，但后续访问必须看到 `NeedsRebuild` 或
  其它明确的 conservative 状态。

## 34. 非目标与边界

B/C-3 不负责：

- mount option、layer registry 或 mount lifetime 的重新解析；
- VFS dentry/inode identity 的重新发明；
- create、unlink、rename、copy-up、whiteout 写入本身；
- BindingCache 的全局命名和 identity policy；
- 把 underlying raw directory entry 作为 Overlay 对外目录项直接返回；
- 为每个 FD 创建 Overlay 私有 directory file、snapshot 或历史 cache；
- 通过 versioned cache 历史实现强一致的 per-FD directory snapshot。

B/C-3 只负责在既定的 Overlay projection、binding、identity 和 `DIR` 一致性
边界内，维护一个当前可见、可遍历、cookie 稳定的目录 index，并把它正确
接入 readdir 输出。

## 35. 设计检查点

接受该设计前，必须能够回答以下问题：

1. upper-only 和 merged directory 是否都使用同一个 Overlay cookie namespace；
2. `readdir_at` 的返回增量是否始终与下一 continuation cookie 一致；
3. 删除、whiteout、lower fallback 和重新出现是否不复用旧 cookie；
4. BindingCache 的 hidden/negative state 是否能支撑 whiteout 和 barrier
   更新，而不把隐藏项放入可见 ReaddirIndex；
5. mutation 是否在 namespace commit 后、释放 parent `DIR` 前更新或标记
   index；
6. index 增量更新失败时是否绝不发布 partial result；
7. impure 是否只影响 identity projection 和相关优化，不被误用为 merge
   visibility authority；
8. 所有可能 sleep 的 underlying I/O 是否都处于允许 sleep 的 lock/domain
   内，且没有释放后重新取得 `DIR` 的逻辑事务。
