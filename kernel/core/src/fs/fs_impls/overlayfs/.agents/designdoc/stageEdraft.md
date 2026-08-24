<!-- SPDX-License-Identifier: MPL-2.0 -->

# Overlay FS 基础设计：Stage E Core Workflow Rehearsal

**状态：** Stage-E 已完成；跨模块闭包已由 Stage-F 确定稿吸收
**范围：** P0/P1 全部 55 个 Micro ID，以及确认随核心实现的 `P2-01 xino`
**定位：** 将 B/C 的模块级语义串成端到端请求流程，不冻结 Rust API、helper
或 Creator pass

## 1. Stage E 的目标和输入

Stage B/C 已经分别确定了 mount、projection、readdir、copy-up、metadata、
mutation 和 advanced seam 的 owner、生命周期、锁边界、publication 规则以及
失败处理。Stage E 不重新定义这些规则，而是检查它们在一条完整请求路径上是否
能够闭合。

所有基础工作流都遵循同一个跨模块骨架：

```text
pin mount/layer and observe current projection
    -> Overlay-local preflight
    -> acquire required Overlay consistency domain
    -> underlying observation or physical operation
    -> physical publication
    -> semantic authority/binding publication
    -> ReaddirIndex update or NeedsRebuild
    -> release locks and references
```

其中：

- lookup、readdir 和 namespace mutation 在相关 parent `DIR` 内完成观察和
  semantic publication；pure file I/O 不反向获取 `DIR`，只在 copy-up trigger
  需要时取得相应的 authority/coordination domain；
- lock order 为 `DIR -> CUL -> INODE -> WL -> UPPER`；
- `DIR` transaction 是运行时语义一致性域，不是跨对象持久化事务；
- 等待、BIO、callback 或释放锁后必须重新验证 mount lifetime、source identity、
  authority、binding、barrier、policy 和 index state；
- physical upper state 在完整 semantic publication 前不能成为 lookup/readdir
  的可见 source；
- Overlay 不拥有第二套真实数据或 page cache，underlying filesystem 仍拥有
 真实对象、数据、普通 metadata 和 durable truth。

## 2. 工作流总览

| 工作流 | 主要入口 | 主要 owner | 基础结果 |
| --- | --- | --- | --- |
| mount 与 root publication | `FsType::create` / mount attach | B/C-1、B/C-2 | 发布完整 Overlay mount 和 root carrier，或失败退出 |
| lookup、stat、readdir | path lookup、stat、directory read | B/C-2、B/C-3 | 发布 positive/negative visibility 和完整可见目录序列 |
| open、read、write-triggered copy-up | file open、I/O、mmap | B/C-4、B/C-5 | 选择 lower/upper authority，必要时完成 copy-up |
| metadata、permission、xattr | setattr、time、permission、xattr | B/C-5、B/C-4 | 先做 Overlay-local check，再操作当前 real authority |
| create、whiteout、unlink、rmdir | directory mutation | B/C-6、B/C-2、B/C-3 | 改变 upper namespace 并发布 visibility/barrier/index |
| link、hardlink | link operation 或 lower target promotion | B/C-4、B/C-6 | 在 upper 建立 link；无 index 时保留明确降级 |
| rename 与 `EXDEV` | same-dir/cross-dir rename | B/C-6、B/C-4 | upper rename，或在任何 side effect 前返回 `EXDEV` |
| fsync 与 teardown | file/fsync、unmount/cleanup | B/C-1、B/C-4 | 转发 upper durability，停止新操作后释放 runtime |

## 3. Mount 与 root publication

### 3.1 正常路径

mount 从 options 和调用上下文取得 lower、upper、workdir、read-only、credential、
durability、identity 和 xino policy。B/C-1 依次：

1. 解析并固定有序 layer stack 和各层 lifetime pin；
2. 验证 upper/workdir 的目录、同 filesystem、可写和 capability 条件；
3. 计算 effective read-only 和 durability policy；
4. 登记 upper/workdir exclusivity；
5. 准备 workdir runtime 状态；
6. 确定 layer identity、mount identity、fsid 和已确认的 xino policy；
7. 准备 root projection inputs；
8. 执行必要的 UUID/private metadata 持久化；
9. 一次性发布 Ready `OverlayMount`；
10. 通过已准备的 root inputs 创建 VFS root 并完成 mount attach。

`root_inode()` 只消费已经准备好的 root inputs，不首次解析 layer、检查 workdir
或执行新的 fallible mount 操作。xino 在这里确定 identity policy 和 fallback，
但具体 `st_ino/d_ino` 只在后续 identity projection、stat 和 readdir 中产生。

### 3.2 失败路径

在 Ready mount 发布前，按照反向依赖释放 root 临时状态、持久化准备状态、workdir
准备状态、upper/workdir claim 和 layer 引用。workdir 清理或 UUID xattr 写入一旦
产生 durable side effect，失败后不承诺通用持久化 rollback。

Mount attach 成功后，错误属于运行期 teardown，而不是 mount construction rollback。
teardown 先阻止新操作，等待 pinned operation 结束，再释放 mount runtime 和
upper/workdir claim。

### 3.3 已确定与延后

- 已确定：没有合法 upper/workdir 的 mount 只读；upper/workdir 是共享资源，不能
  被两个 Overlay mount 同时使用；root publication 必须是完整 carrier；
- 已确定：xino policy 随核心 identity policy 进入 mount；
- 延后：完整 UUID mode、P3-09 workdir/index residue cleanup 和 cache-miss
  callback 的具体 VFS adapter 形状；basic lookup 正确性不依赖 VFS private
  payload。

## 4. Lookup、stat 与 readdir

### 4.1 Lookup 主路径

一次 named lookup 取得 pinned mount/layer references 和 Overlay parent `DIR`，
并在该 consistency domain 中完成 BindingCache-first 流程：

1. 先查询 `(parent, name)` 的 BindingCache；
2. cache hit 时直接使用已发布的 positive/negative projection；
3. cache miss 或 stale 时读取 upper 和按顺序排列的 lower observations；
4. 按 upper-first 规则做 visibility reduction；
5. 将普通对象、merged directory、absent、whiteout-hidden 和 opaque-hidden
   分成明确的 positive/negative result；
6. 对 positive result 执行 ID projection；
7. 在同一 `DIR` 事务内更新 BindingCache、barrier、identity 和必要的
   ReaddirIndex；
8. 返回 inode 或 `ENOENT`，由 VFS 形成派生的 positive/negative dentry cache；
9. 释放 upper protection、Overlay `DIR` 和临时 observations。

whiteout 和 opaque 只提供 visibility barrier/evidence，不能生成伪 inode。普通
lookup 默认保持一个 Overlay parent `DIR` 跨越必要的 underlying BIO；只有已经
证明的同步重入或反向锁序才允许局部 lock-neutral/retry。

### 4.2 stat 与 xino

stat 使用当前 positive binding 和 logical object authority，不通过 `st_ino` 反查
路径。`P2-01 xino` 随核心实现时：

- `st_dev/st_ino` 从 mount/layer-qualified object identity policy 投影；
- `d_ino` 与同一 identity policy 保持一致；
- copy-up 只改变 real authority/provenance，不重新创建 logical inode、name
  binding 或已经暴露的 directory cookie；
- xino 不提供 `ino -> name`，也不替代 origin/index 的 lower-upper 关联；
- xino overflow 的具体处理不在 Stage-E 展开，后续直接参考 Linux 源码完成
  具体设计；这不影响当前把 xino 作为核心 identity policy 的决定。

### 4.3 Readdir 主路径

readdir 在 parent `DIR` 内读取或重建当前 `ReaddirIndex`：

1. 若 index 为 `Valid`，按 VFS offset 查询下一个可见 entry；
2. 若 index 未建立或为 `NeedsRebuild`，按 upper-first、layer order、whiteout、
   opaque 和 dedup 规则形成完整可见序列；
3. 对每个可见 entry 投影 `d_ino`，不暴露 underlying raw cookie；
4. 只有完整序列形成后才发布新的 index；
5. 返回当前 cursor delta，并保留 monotonic、never-reused cookie 语义。

namespace mutation 在释放 parent `DIR` 前更新受影响的 ReaddirIndex，或将它标为
`NeedsRebuild`。partial upper observation 或 partial index 不能被发布。

## 5. Open、read 与 write-triggered copy-up

### 5.1 统一请求路径

文件请求先依据已发布 projection 判断 object kind，再判断 operation intent：

```text
VFS request
    -> object kind and read/write intent
    -> Overlay-local permission/read-only check
    -> copy-up trigger or fast path
    -> current real file/directory handle
    -> underlying operation
    -> release view/handle
```

read-only open、read、seek、read splice、stat 和普通 readdir 不触发 copy-up。write
open、write、truncate、fallocate、write splice 和 writable shared mmap 必须先让
目标取得 upper authority。metadata mutation 走 full-data copy-up 的基础路径；
metadata-only copy-up 属于后续 metacopy 扩展。

### 5.2 Regular-file full copy-up

copy-up trigger 通过 entry-scoped coordination 选择一个 winner；其他 waiter 等待
后依据 fresh authority 重试。winner：

1. 确认 parent/target projection、mount lifetime 和 permission policy 仍有效；
2. 取得 `CUL`，必要时递归准备 upper parent；
3. 在 workdir 建立 private temporary object；
4. 使用 creator credential 完成 metadata、eligible xattr、origin 和 full data
   transfer；
5. 执行规定的 durability handoff；
6. 将完整 temporary physical object 发布到 upper；
7. 在同一语义边界内切换既有 logical object 的 authority/provenance；
8. 更新或失效 binding、identity、page-cache view 和相关 directory state；
9. 释放 `CUL`，让调用者取得当前 upper handle。

authority-only copy-up 不重新编号 inode、name 或 cookie。page cache 继续属于
当前 underlying inode，Overlay 不建立第二个 page-cache backend；write-capable
mapping 和 upper page-cache inlet 必须在 trigger 完成后才开放。

### 5.3 失败路径

physical publication 前的失败清理 workdir temporary，并保留 lower authority。若
physical publication 已完成但 semantic publication 失败，不宣称通用 rollback；
应保守地失效 binding/index/cache state，通过重新观察和 reconcile 收敛。

## 6. Metadata、permission 与 xattr

所有基础 metadata 操作共享一条两步管线：

```text
Overlay-local read-only/type/credential check
    -> obtain current real authority through B/C-4
    -> underlying creator-credential check
    -> underlying metadata/xattr operation
    -> publish authority/cache changes if visible state changed
```

本地检查失败时不能进入 copy-up，不能创建 workdir temporary 或产生 upper side
effect。真实 handle 或 copy-up 已经开始后，底层 creator-credential 检查失败由
B/C-4 负责 transition cleanup/reconcile，B/C-5 不重新定义 authority 生命周期。

private/public xattr 需要在本地检查后增加分类和 private-owner authorization：

- public xattr 按当前 real authority delegation；
- private xattr 由 Overlay policy 选择 namespace、过滤规则和 owner；
- private record 需要 underlying access 时，仍回到 B/C-4 的 real-handle seam；
- whiteout、opaque、origin 等 private record 不能通过普通 `listxattr` 暴露。

atime/mtime/ctime 更新、chmod/chown/utimes 和基础 xattr 操作都必须遵守同一条
local-first 管线。`default_permissions` 只改变 underlying second check 的 policy，
不取消 Overlay-local check。

## 7. Create、whiteout、unlink 与 rmdir

### 7.1 Create

在 parent `DIR` 内重新确认 target visibility、parent upper readiness、mount
writability 和权限后：

- truly absent 且无冲突时，直接在 upper parent 创建 object；
- create-over-whiteout 时，在 workdir 准备完整 object，再原子替换 whiteout；
- 创建 directory over hidden lower directory 时，opaque record 成为完整
  replacement publication 的一部分；
- 创建新 symlink 只创建新 upper symlink，不 copy-up referent。

成功后发布 binding、identity、barrier 和新 cookie；失败时不能先删除 whiteout
再暴露未完成的新 object。

### 7.2 Unlink 与 rmdir

- pure-upper object：直接删除 upper object，通常不创建 whiteout；
- upper-over-lower object：删除 upper 后发布 whiteout，继续隐藏 lower；
- lower-only object：不修改 lower，只在 upper parent 创建 whiteout；
- rmdir：按 Overlay-visible emptiness 检查，不能只扫描 upper children；
- whiteout-hidden child 不算可见 child，visible lower/upper/merged child 仍使
  rmdir 失败。

physical upper operation 成功后，在 parent `DIR` 释放前更新 BindingCache、
barrier、identity 和 ReaddirIndex；更新失败则使用 `NeedsRebuild`/revalidation，
不返回一个与当前 upper truth 不一致的成功结果。

## 8. Link、hardlink 与 no-index 降级

`P1-28 link` 的基础操作是：先确认 source 的当前 authority，再让 lower target
必要时取得 upper authority，最后在 upper parent 执行 link 并发布 target binding。

无 index 时必须明确区分：

- upper-authoritative source 的新 link 与已有 upper name 共享同一个 upper inode；
- lower 中原本多名指向同一 real inode 的 aliases，在分别 copy-up 时不保证继续
  指向同一个 upper inode；
- xino 维持 identity projection，但不能提供 lower-object → upper-inode 的持久
  关联；
- `P2-07` 若未来加入，只改善 `st_nlink`/bookkeeping，不恢复已经断开的物理
  alias 关系。

因此 basic Overlay FS 保证 upper-side hardlink operation 的正常语义，但不承诺
no-index lower multi-link relationship 跨 copy-up 的全局保持。`P3-01 index` 是
后续关闭这项降级的扩展，不改变基础 link workflow 的 owner。

## 9. Rename 与默认 `EXDEV`

rename 先在一个或两个 parent `DIR` consistency domain 内重新观察 source、target、
object kind、whiteout/barrier 和 upper readiness。跨 parent 时按稳定 identity
顺序取得两个 `DIR`，同一 parent 只取得一次。

- upper-authoritative、同目录、non-directory rename：在 upper 执行 rename，必要
  时处理 target whiteout/replace，再发布 source/target binding 和两个 directory
  index；
- lower-origin non-directory 或纯 upper directory：按基础 copy-up/promotion
  规则准备 upper 后执行允许的 rename；
- lower-backed 或 merged directory 的跨目录 rename：在任何 copy-up、redirect
  xattr 或 upper side effect 前返回 `EXDEV`；
- target whiteout、source whiteout 和 opaque/barrier 的变化必须和 physical
  rename 一起进入 semantic publication。

`P2-02 redirect_dir` 未来只扩展第三种路径：directory promotion、redirect xattr
和后续 lookup interpretation。没有启用 redirect policy 时，它不能改变 basic
`EXDEV` 结论。

## 10. Fsync、cleanup 与 teardown

### 10.1 Fsync

基础 `fsync` 取得当前 real file view，并把数据/metadata sync 请求转发给
underlying upper/lower authority。Overlay 只负责按 mount policy 选择正确 real
object、保证 authority 已经稳定，并在 copy-up 后同步当前 upper file；它不承诺
跨多个 underlying object 的通用事务。

基础实现保留默认 delegation/`auto` 语义。`strict` 的额外 parent-directory sync、
volatile mount 和其他 durability mode 属于后续范围；它们不能改变 Stage E 的
publication/reconcile 规则。

### 10.2 Teardown

teardown 的正常顺序是：

1. 阻止新的 Overlay 请求进入；
2. 等待仍持有 mount/layer pin、real handle 或 copy-up coordination 的操作结束；
3. 释放 VFS root/private bindings、inode/binding cache 的可回收引用；
4. 释放 upper/workdir exclusivity 和 mount runtime；
5. 按明确策略处理 workdir/index residue，并报告 cleanup failure。

运行时引用释放可以依赖 RAII，但 workdir temporary、UUID、private xattr、index
record 等 durable side effect 的清理不能由 `Drop` 隐藏或假设自动回滚。`P3-09`
cleanup 是未来恢复增强，不是 basic teardown 成功的前提。

## 11. 跨工作流闭包检查

| 交接 | 已确定的闭包 |
| --- | --- |
| mount → lookup | mount commit 先发布 layer snapshot、identity policy 和 root carrier；lookup 只消费已发布输入 |
| lookup → stat/readdir | BindingCache 和 ReaddirIndex 是第一信源，name/ID projection 与两者使用同一 visibility result |
| lookup → copy-up | copy-up 保留 logical inode/name/cookie，只切换 authority/provenance |
| permission → copy-up | local failure 先截断副作用；real check 在 copy-up seam 内使用 creator credential |
| copy-up → file I/O | trigger 完成后才能取得 writable real handle 或 upper page cache |
| mutation → lookup/readdir | physical success 后在同一 `DIR` 内发布 binding/barrier/identity/index |
| rename → `EXDEV` | 默认在 upper side effect 前拒绝 lower/merged directory cross-dir rename |
| index/identity → NFS | index/origin verification 和 cleanup 先于未来完整 NFS export |
| teardown → persistent cleanup | runtime lifetime 与 durable cleanup 分离，cleanup failure 不伪装成 rollback |

## 12. 后续实现与扩展事项

这些项目不阻止 Stage-E 的语义稿完成，留给实现前的局部决策或后续扩展设计：

- 完整 UUID mode 的最终策略；
- upper filesystem 对 whiteout、opaque、rename/exchange、xattr 和 durability
  capability 的实际探测接口；
- no-index hardlink 降级下 `st_nlink` 的基础报告策略；
- workdir-only mount、强制只读 upper claim 和跨 mount inuse carrier 的最终
  implementation placement；
- `redirect_dir`、metacopy、index、NFS export 的后续 extension contracts。

## 13. Stage-E 结论

Stage E 的核心工作流已经可以从 B/C 的已确认语义确定地写出。它没有新增
ownership、lock topology、publication law 或实现 pass。完成本稿后，后续工作
转向 Stage F 的 Meso responsibility/interface decomposition；Micro traceability
和 xfstests evidence mapping 继续由已接受的 Architect/Designer artifacts 提供，
不再拆分额外的设计阶段。
