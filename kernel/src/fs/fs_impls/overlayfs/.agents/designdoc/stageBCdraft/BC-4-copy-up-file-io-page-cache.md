<!-- SPDX-License-Identifier: MPL-2.0 -->

# B/C-4：Copy-up、File I/O、Page Cache

**状态：** 设计完成，交互式讨论结论已纳入

**对应 meso-components：**

- `copy_up_lifecycle`
- `file_io_page_cache`

**基础主线覆盖 Micro-feature：**

- `P1-01` 至 `P1-07`：copy-up trigger、协调、parent preparation、workdir
  temporary、metadata/data/xattr/origin transfer 和 publication；
- `P1-08` 至 `P1-15`：file open/release、read/write、llseek、mmap、fsync、
  fallocate/fadvise 和 splice delegation；
- `P1-34`：workdir temporary helpers；
- `P1-37`：page-cache forwarding 和 copy-up trigger。

**条件扩展：** `P2-08`（copy/remap）、`P2-09`（fiemap）和 `P2-10`
（file/page-cache hooks）与本阶段有接口耦合，但是否进入基础实现仍留给
Stage D 决定。本稿先说明它们必须服从的 authority 选择规则，不把它们当作
已经确认的初始范围。

**边界补充：** 本稿同时收敛 hardlink、symlink 的 object-kind copy-up 规则，
以及 lower hardlink 与 `PersistentOriginIndex` 的关系。实际的目录命名空间
mutation 仍由 B/C-6 持有；本节只规定 copy-up 如何取得或复用 upper object，
不改变已接受的 Micro-feature ownership。

本节也为 `P1-28` 的 lower-target copy-up side、`P1-32` 的 symlink object-kind
read/copy-up side 和 `P2-07` 的 nlink projection 提供跨模块说明；这些 Micro
feature 的正式 owner 不因本节的叙述位置改变。

**前置：** B/C-1 提供已发布的 mount、upper/workdir lifetime、read-only、
creator credential 和 durability policy；B/C-2 提供 object projection、
identity carrier、BindingCache 和 parent `DIR` 一致性域；B/C-3 提供
ReaddirIndex 以及目录 namespace 变化后的索引维护规则。

## 36. 先看一条完整请求路径

B/C-4 不从一组孤立的 carrier 或锁开始，而从“一个 VFS 请求如何到达
underlying filesystem”开始。对一个已经由 B/C-2 投影出来的 Overlay object，
统一路径是：

```text
VFS request
    -> 1. 判断 object kind：directory 还是 non-directory
    -> 2. 判断 operation intent：read-only 还是需要写 authority
    -> 2.5. 执行 Overlay 本地权限和 read-only 检查
    -> 3. 进入 copy-up trigger：必要时执行或等待 copy-up；不必要则 fast path
    -> 4. 根据结果取得实际 real-file / real-directory handle 或 projected view
    -> 5. 把操作转发给选定的 underlying object
    -> 6. 按 VFS 语义释放 view/handle
```

这里的关键是第 3 步：**copy-up 是一个 trigger/authority-transition module，
不是每个文件操作都要调用的通用数据处理函数。** 每个可能写入的请求都要
先经过这个 trigger；trigger 可以判断“已经是 upper”“需要完整 copy-up”或
“这个目录写操作应该走 whiteout/`EXDEV`，不复制目标对象”。只有 trigger
完成后，第 4 步才可以选出实际句柄，第 5 步才可以发生。

### 36.1 第一步：判断 object kind

object kind 使用 B/C-2 已经投影出的 Overlay type 和 binding 事实，不重新从
raw underlying entry 猜测。这里至少区分：

- **regular/non-directory object：** 需要 regular-file data I/O、mapping 或
  metadata transfer 时，可能进行 file copy-up；symlink、device 等对象使用
  各自的 object-kind transfer 规则，不把它们误当作 regular file；
- **directory object：** read/readdir 使用 merged 或 upper-only directory
  projection；namespace mutation 可能需要提升 parent/directory，但通常不
  复制目录内容。

这一步决定 copy-up 的 recipe。它不是简单地把所有对象都交给同一个
`copy_file` 流程。

### 36.2 第二步：判断 operation intent

intent 不是由调用者传入的一个永远可信的 flag，而是 Overlay 在 authority
选择前根据 VFS operation、access mode、mapping mode 和 mount policy 得出的
语义判断：

| 请求类别 | 基础处理 |
| --- | --- |
| read-only open、read、seek、read splice、普通 stat/readdir | 不触发 copy-up，使用当前 lower/upper source 或 directory projection |
| write-capable open、write、truncate、writable shared mmap | 先触发对应 object 的 copy-up，再取得 upper handle |
| fallocate、write splice、其它 data mutation | 先触发 file copy-up；P2 range 语义仍待范围决策 |
| chmod/chown/utime 等 metadata mutation | 基础设计先执行 full copy-up，再对 upper metadata 操作；metadata-only copy-up 留给 B/C-7 |
| create/mkdir/link/unlink/rmdir/rename | 先触发 parent/directory 侧的 upper preparation；是否复制 target 由具体 namespace 操作决定 |
| 创建一个新 symlink | 不为了创建这个新名字而 copy-up 一个既有 lower object |

因此“任何写操作先触发 copy-up”应理解为：**任何可能改变 Overlay 可见状态
的写请求先进入 trigger 判定**，而不是“每个请求都复制它操作的目标实体”。
例如 unlink lower file 通常需要在 upper parent 创建 whiteout，而不是把被删的
lower file 复制到 upper 后再删除；rename lower/merged directory 在基础范围内
可以直接返回 `EXDEV`，而不是隐式启用 redirect copy-up。

本地权限检查必须发生在 trigger 产生任何 upper/workdir side effect 之前。它
检查当前任务对 Overlay object/parent 的权限、mount read-only policy 以及
operation intent。copy-up 内部还会通过 underlying filesystem 执行 real check：
使用 mount 保存的 creator credentials 检查 lower source 的读取，以及
upper/workdir 的创建、写入、xattr 和 rename 权限。copy-up 不重新定义权限
模型；它消费外层的授权结果，并传播底层 real check 的失败。

### 36.3 第三步：必要时执行 copy-up，随后才取得实际句柄

trigger 的结果不是“已经完成了用户请求”，而是一个 authority decision：

- **No copy-up：** 当前 real object 已经是 upper，或者请求只读且 lower 足够；
- **Upper ready：** lower-to-upper transition 已完成，调用者可以取得 upper
  real handle；
- **Directory route：** directory operation 已取得 upper parent/dir，或者被
  路由到 merged readdir、whiteout 或 `EXDEV` 语义；
- **Wait/retry/error：** 另一个请求正在转换、转换失败需要重新观察，或
  mount/upper/workdir/权限条件不满足。

调用者不能在 trigger 返回前缓存一个 lower file handle，然后把它当作后续
write 的目标。等待、BIO 或 callback 返回后，必须根据最新 authority 再取得
一次实际 handle。

### 36.4 第四步：转发并释放

获得实际句柄后，Overlay 只增加 authority、credential 和 read-only policy
所要求的边界，然后转发 underlying 的正常语义：short I/O、offset、错误、
mapping、fsync 和 release 不应被 Overlay 重新发明。

返回给 VFS 的 file view 或 directory view 必须保持相应 real reference 的
lifetime，并在 release 时只释放一次。Overlay lock 不能通过返回值泄漏到
VFS，也不能用 release 作为隐式的 authority lock。

## 37. Copy-up trigger module 的大致形状

### 37.1 它解决什么问题

copy-up trigger module 的唯一核心问题是：

> 当前请求是否需要让这个 logical object 取得 upper authority？如果需要，
> 哪个请求负责完成转换，其他请求如何等待并重新取得结果？

它不负责最终的 read/write/llseek，也不拥有目录 index、whiteout、origin
export 或 page cache。它只是把“lower 还不能接受这个操作”转换成“upper 可以
接受这个操作”的入口。

### 37.2 概念上的入口和结果

下面是责任形状，不是冻结的 Rust signature 或 helper 名称：

```text
ensure_authority(
    projected object,
    operation intent,
    object kind,
    mount policy,
    caller context,
) -> Result<()>
```

入口内部应由 `OverlayCopyUpState` 这样的稳定责任主体承载，而不是由每个
file operation 维护一套局部布尔值。它消费的是 B/C-2 已有的
`OverlayInode`、`OverlayObjectState` 和 binding 的持久引用；不应额外产生一个
短生命周期的 `AuthorityDecision` carrier。结果至少能表达：

- 当前 authority 可直接使用；
- 需要对此 object 执行 file copy-up；
- 需要对此 directory 执行 directory promotion/parent preparation；
- 这个 namespace 写操作走 whiteout 或 `EXDEV`，不复制 target；
- 另一个 transition 正在进行，当前请求需要等待并重试；
- operation 被 read-only、权限、upper/workdir、底层 I/O 或一致性错误拒绝。

trigger 返回的是“可继续使用的 authority 结果”，不是必须立即返回一个
具体的 VFS `File` 类型。实际 handle 的选择仍由 file/directory delegation
路径完成，这样 copy-up module 不会反过来拥有所有文件操作。

更严格地说，trigger 的语义结果应体现在持久 owner 上，而不是体现在一个
临时 enum 上：`OverlayObjectState` 发布 lower/upper authority，binding 发布
当前实际 binding，调用者随后基于同一个 owner 取得 real handle。具体的
`OverlayCopyUpState` 是挂在 binding、inode 还是由两者共同引用，暂不在本阶段
冻结；但它不能只是一次 trigger 调用的栈变量。完成状态由 upper binding 和
object authority 表达，不需要长期保留一个 `copy-up completed` 历史标记。

### 37.3 Fast path、winner、waiter

trigger 的常见控制流是：

```text
read current object authority
    -> already upper? return upper-ready
    -> request read-only? return lower-ready
    -> request needs upper?
         -> acquire this object's copy-up coordination
         -> became upper while waiting? return upper-ready
         -> become winner and execute the object-kind recipe
         -> publish result, wake waiters, return upper-ready
```

同一个 lower-backed object 只能有一个 live winner。waiter 不能持有不兼容的
`INODE`/`UPPER` 状态等待；醒来后必须重新检查 object authority、lower source
identity、upper parent 和 operation intent。winner 失败后，waiter 只能从
fresh lower observation 重试。

winner 从取得 `CUL` 开始，持续保持该 copy-up coordination ownership，直到
成功完成 semantic publication，或完成失败清理并确定 retry/reconcile 结果。
因此 winner 不需要在每个 BIO、metadata 或 workdir 步骤之后轮询
“是否已经 ready”。正常检查点只有：取得 `CUL` 后的竞争复查、各阶段底层
操作的错误检查、最终 publication 检查。waiter 在等待结束并重新取得协调
状态后检查一次最终结果即可。

如果某个 underlying callback 可能重入 Overlay 并重新取得当前锁，才允许走
lock-neutral handoff；释放锁后返回必须重新验证 authority。这是已证明的
特殊 callback 边界，不是普通 non-reentrant BIO 的默认流程。

trigger state 是转换期间的协调事实，不是第二个 durable namespace。它需要
能区分至少以下语义状态：

```text
Lower authoritative
    -> Copy-up in progress
    -> Upper authoritative

Copy-up in progress -> retryable failure
Copy-up in progress -> reconcile required
Upper authoritative  -> idempotent fast path
```

`reconcile required` 只表示 physical upper step 已发生但 Overlay 还没有完成
语义确认；它不能被当作成功，也不能让下一个 lookup 无条件采用一个 raw upper
entry。

### 37.4 Trigger 不等于覆盖后续 file I/O 的大锁

trigger 需要在 copy-up 阶段持续持有 `CUL`，但不把后续的 `read`、`write`、
page fault、readdir 和已经取得 actual handle 的 delegation 都塞进同一个
锁。`CUL` 覆盖的是 winner 的 lower-to-upper transition，不覆盖用户操作的
整个生命周期。概念上只需记住：

- read-only request 可以绕过 copy-up，并使用 pinned lower source；
- write request 必须先穿过同一个 trigger；
- trigger 完成后 file operation 才取得 actual handle；
- actual handle 的 underlying I/O 仍由 underlying filesystem/page cache 处理。

## 38. 普通文件的 copy-up

这里的“文件 copy-up”主要指 lower-backed regular file 因为写入或 metadata
mutation 而需要 upper authority。基础范围建议采用 full-data copy-up；
metacopy/verity/data-only 等延迟数据路径放到 B/C-7。

### 38.1 File copy-up 的步骤

```text
lower file projection
    -> 触发并赢得 copy-up coordination
    -> 确保 upper parent 和 ancestor
    -> 在 workdir 创建 private temporary
    -> 将 metadata、eligible xattrs、origin 和 data 写入 temporary
    -> 按 mount durability policy 完成必要同步
    -> 将 temporary 原子发布到 upper target name
    -> 更新 Overlay object/binding authority
    -> 取得 upper real-file handle
    -> 转发 write/mmap/metadata operation
```

具体顺序仍要满足以下约束：

1. upper parent/ancestor 先于 child publication 准备好；
2. temporary 在 workdir 内完成 object kind、owner/mode/timestamps、size、
   eligible user xattrs/ACL metadata、origin identity 和 regular-file data；
3. overlay-private xattrs 不作为普通用户 metadata 复制；
4. 所有需要在 publication 前完成的 data、metadata、origin 和 durability
   检查都在 temporary 阶段完成；
5. physical publication 和 Overlay semantic publication 之间不能让新的
   lookup/open 看到一个 incomplete upper authority；
6. publication 后的 file operation 只取得 upper handle，不再使用旧 lower
   selection 进行写入。

其中本地 Overlay permission check 已在 trigger 前完成；temporary 的创建、
source data 读取、upper 写入、xattr/origin 操作和最终 namespace 操作仍分别
接受 underlying filesystem 的 creator-credential real check。任何一项失败都
终止本次 winner 流程，不把不完整 temporary 当作可用 authority。

### 38.2 哪些文件操作触发 file copy-up

| 操作 | 为什么需要 copy-up | copy-up 后的实际 handle |
| --- | --- | --- |
| read-only open/read | 不改变 lower，通常不需要 | lower real file |
| write-capable open | VFS 可能随后写入或 truncate | upper real file |
| write/write splice | lower 不能作为 Overlay 写入目标 | upper real file |
| writable shared mmap | mapping 可能修改 file data | upper real file/page-cache relationship |
| fallocate 或其它 data mutation | 改变数据块或文件 size | upper real file |
| chmod/chown/utime | 基础设计保证 copy-up 前后权限/metadata 一致 | upper real file |
| read-only mmap | 保留 lower mapping divergence | pinned lower real file |

已经 upper-authoritative 的 regular file走 idempotent fast path，不再创建
第二个 temporary。一个已经打开的 lower read-only file view 或 read-only
mapping 也不因后来发生 copy-up 而自动迁移；它只在原来的 pinned lower
relationship 上完成自己的 read。

### 38.3 File copy-up 的失败边界

在 temporary 尚未发布前失败：lower 继续是 authority，temporary 尽力清理，
失败结果唤醒 waiter 并允许 fresh retry。若 cleanup 失败，残留 temporary
仍不得进入 Overlay namespace，必须保留为明确的 workdir cleanup/reconcile
obligation。

physical upper publication 已发生但 runtime publication 尚未完成：不能声称
copy-up 成功，也不承诺通用 rollback。trigger 进入 reconcile path，验证
upper object 的 type、origin、metadata 和 target binding；只有语义确认完成后
才唤醒 waiter 让其取得 upper handle。

### 38.4 Hardlink、symlink 与 PersistentOriginIndex

#### 38.4.1 只读访问不触发 copy-up

只读访问 lower hardlink 或 symlink 都不因为“对象存在于 lower”而触发
copy-up：

- 读取 hardlink 的任一名字、`stat` 或只读打开，继续使用 lower real object；
- 读取 symlink 或跟随 symlink，继续使用 lower symlink 的语义；
- lookup 可能查询已经存在的 `PersistentOriginIndex` 并建立内存中的
  authority/identity 关联，但这不是 copy-up，也不产生 upper namespace
  mutation。

这里的只读语义不排除 underlying 文件系统自身的 atime 规则；atime side
effect 不应被误认为是 copy-up。

#### 38.4.2 Hardlink 的两种 copy-up 形态

lower 和 upper 通常不是同一个文件系统，因此不能把 lower inode 直接 link
到 upper。首次 copy-up 一个 lower hardlink alias 时，基础操作是：

```text
创建 upper inode
复制 lower 对象的数据和需要保留的元数据
在 upper parent 创建第一个可见目录项
```

如果已经有对应的 upper inode，后续 alias 的 copy-up 不再复制数据，而是
在 upper parent 中增加一个指向既有 upper inode 的目录项，等价于：

```text
link(existing_upper_inode, upper_parent, new_name)
```

用户显式调用 `link()` 时也遵循同一原则：先让 source 成为
upper-authoritative，再在 upper 中创建 hardlink。source 的 copy-up 由
B/C-4 提供，source/target parent 的 namespace transaction 和最终目录项
发布由 B/C-6 负责。

#### 38.4.3 Index 是 origin 映射，不是 copy-up 必经步骤

`PersistentOriginIndex` 用 lower origin 标识把同一个 lower inode 映射到同一
个 upper inode：

```text
upper/index/<lower-origin> -> upper inode Y
upper/a                    -> Y
upper/b                    -> Y
```

这样 lower 中原本互为 hardlink 的多个名字，在分别发生 copy-up 后仍能复用
同一个 upper inode。index 不只是首次 copy-up 时创建的记录；后续 lookup、
hardlink、unlink/rename 生命周期和必要的清理都可能消费这条映射。

但并非每次首次 copy-up 都创建 index：

- `index=off` 时不创建；
- `index=on` 但 lower 对象是普通单链接非目录时通常不创建；
- `index=on` 且未启用 NFS export 时，目录通常不因普通 promotion 创建；
- 启用 NFS export 时，index 可能扩展到所有需要稳定 export identity 的
  lower 对象。

因此，copy-up 是 lower-to-upper authority transition，而
`PersistentOriginIndex` 是在需要 origin/hardlink/export identity 时附加的
持久化映射。

#### 38.4.4 Index 与联合 nlink

对普通文件，隐藏 index entry 本身也是 upper inode 的一个 hardlink，因此
underlying upper `i_nlink` 会包含这条隐藏链接；但 overlay 对外报告的
`nlink` 不能把它算作可见名字。启用 index 的联合语义可以概括为：

```text
overlay nlink
    = upper 中可见 hardlink
    + lower 中尚未被 upper 覆盖的 hardlink
```

因此 index 的作用是提供稳定的 origin/inode 锚点，让 Overlay 能判断哪些
upper/lower 名字属于同一 hardlink group；`.overlay.nlink` 则保存相对 lower
或 upper 当前 nlink 的差值。index 是身份和 bookkeeping 的载体，不是单独
的 nlink counter。

#### 38.4.5 Symlink 的 object-kind recipe

普通 symlink 的 copy-up 只复制 symlink 自身：读取 lower symlink 的 target
字符串，在 upper 中重新创建相同 target 的 symlink，并复制适用的 metadata。
它不会打开、跟随或复制 target 指向的 regular file/directory。

symlink read 不需要 promotion；只有修改 symlink 自身的 metadata/xattr，或
其它明确要求 upper authority 的操作，才进入 copy-up trigger。若 symlink
本身拥有多个 hardlink，则它仍受上一节 hardlink group 规则约束；这不是
symlink 独有的 index 语义。

## 39. 目录的 copy-up 与 directory operation

目录不能照搬 regular file 的“复制所有 data”流程。目录 copy-up 的核心是
**提升 upper namespace 的目录承载能力**，而不是复制 lower directory 的
全部 children。

### 39.1 目录读操作：通常不 copy-up

以下操作只读目录 projection，不需要为了读取而创建 upper directory：

- lookup/stat 一个 lower 或 merged directory；
- open directory；
- readdir；
- 判断目录是否为空或读取可见 children。

merged directory 使用 B/C-3 的 ReaddirIndex；upper-only directory 可以把
source read 委托给 upper，但 raw underlying cookie 不能直接暴露给 VFS。对
merged directory 来说，第 4 步取得的不是单一 underlying directory handle，
而是由 Overlay projection、BindingCache、barrier state 和必要的 source
handles 组成的 `DirectoryView` 语义结果。

### 39.2 目录写操作：先处理 parent/target 角色

目录写操作需要先经过 trigger，但 trigger 选择的 recipe 依赖操作：

| 目录操作 | 需要的 upper 动作 | 是否 copy-up target directory/file |
| --- | --- | --- |
| 在 lower/merged directory 中 create/mkdir | 提升 parent directory，随后在 upper parent 创建新 entry | 不复制新 target；target 是新建对象 |
| 在 lower/merged directory 中 unlink lower entry | 提升 upper parent，创建 whiteout | 不复制被删除的 target |
| rmdir lower directory | 提升 upper parent，创建正确的 whiteout/barrier | 不复制被删除目录的 children |
| 修改 lower directory 的 mode/owner/time/xattr | 提升该目录的 upper metadata 载体 | 只复制目录 metadata，不复制 children |
| rename lower/merged directory | 基础范围返回 `EXDEV` | 不隐式启用 redirect copy-up |
| rename 已 upper-authoritative entry | 准备 source/target upper parents 后执行 upper namespace mutation | 不重复 copy-up 已 upper target |

这里“提升目录”可以发生在操作目标的 parent，也可以发生在需要保留自身
metadata 的 directory object；必须先明确哪个 object 成为 upper authority。
unlink/rmdir 这类操作的写入对象是 upper namespace 中的 whiteout/barrier，
不是被删除的 lower object。

### 39.3 Directory promotion 的步骤

基础目录 promotion 的语义步骤是：

1. 通过 B/C-2 projection 确认 lower/merged directory identity 和当前 barrier；
2. 递归确保 upper ancestors/parent 已存在；
3. 在对应 upper path 创建一个 directory object；
4. 复制该目录需要保持一致的 owner、mode、timestamps、eligible xattrs 和
   origin metadata，但不复制 lower children；
5. 在 Overlay object/binding 中发布 upper directory authority；
6. 保持 lower children 的 merge 可见性，除非后续 directory mutation 明确
   创建 opaque barrier；
7. 在 parent `DIR` transaction 内更新 BindingCache、barrier state 和
   ReaddirIndex，或在无法局部更新时标记 `NeedsRebuild`；
8. 返回 upper real-directory handle 给 directory mutation module。

目录 promotion 的“publication”仍必须是一个完整的 semantic transition，不能
先让 lookup 看到一个 metadata 尚未完成的 upper directory。由于 directory
没有 regular-file data，基础路径不需要为每个 directory promotion 创建
workdir data temporary；是否需要 underlying-specific temporary/atomic
replacement 留给后续实现与 durability 讨论。

### 39.4 Directory promotion 与 workdir

mount 阶段已经保证 valid upper/workdir pair；但这不表示每次 directory
operation 都要在 workdir 创建临时目录：

- regular-file full copy-up 使用 workdir 作为 private data staging area；
- directory promotion 默认直接创建并准备 upper directory，因为没有需要
  搬运的 regular-file data；
- 如果未来的 metadata/atomic-publication 策略要求 directory temporary，必须
  明确增加对应的 cleanup/reconcile 规则，不能从 regular-file recipe 默认为
  目录复制一份 children；
- workdir 中的任何 temporary 都不能成为 ReaddirIndex 的 source，也不能
  通过普通 lookup 暴露给用户。

## 40. Workdir：copy-up 的临时 staging，而不是第二个文件系统

### 40.1 Workdir 做什么

workdir 的职责可以压缩为三件事：

1. 为 regular-file full copy-up 提供一个不在 Overlay namespace 中的 private
   temporary 位置；
2. 让 data、metadata、xattr、origin 和必要 durability 操作在 publication
   前完成；
3. 在失败、mount teardown 或后续 cleanup 中提供可识别的残留处理位置。

upper/workdir 的 same-filesystem 和 exclusivity 已由 B/C-1/mount policy
   保证；B/C-4 不重新取得或解释这些 mount claims。

### 40.2 Workdir 与实际 file handle 的关系

temporary handle 只属于 winner 的 copy-up transaction。它不是：

- 返回给 VFS 的 file handle；
- OverlayInode 的长期 real-file relationship；
- page-cache forwarding 的目标；
- ReaddirIndex 的目录项。

只有 physical publication 和 semantic publication 都完成后，trigger 才向
后续 file operation 暴露 upper real-file handle。失败时 temporary 关闭并清理；
清理失败也不能把它当成可用 upper。

## 41. Page cache 到底与 copy-up 如何交互

### 41.1 Overlay 不拥有第二个 page cache

Page cache 由实际 underlying inode 负责，Overlay 只负责在 page-cache 入口
之前做 authority 选择：

```text
read-only lower request
    -> lower real inode / lower page cache

writable request
    -> copy-up trigger
    -> upper real inode / upper page cache
```

Overlay 不创建一个同时缓存 lower 和 upper 内容的 Overlay-owned page cache，
也不把 lower page 与 upper page 混在同一个 cache 中。copy-up data transfer
可以使用 underlying file I/O、underlying page cache 或 BIO capability；这些
都是 underlying filesystem 的实现责任，不是 B/C-4 新增的第三套 cache。

### 41.2 Trigger 必须在 page-cache write inlet 之前发生

对于 write open、writable mmap、write-at 或 writeback-capable operation，
copy-up 必须在允许写入或建立 writable mapping 之前完成。不能先把 lower
mapping/page-cache view 发给 VFS，再期待 page fault 时由一个不受协调的
callback 临时 copy-up。

read-only lower mapping 可以保留既定的 lower `MAP_SHARED` divergence：后来
发生 copy-up 不会把已有 mapping 改成 upper mapping。新的 writable mapping
必须重新走 trigger，并绑定 upper real inode 的 cache。

### 41.3 Copy-up 期间的两个 cache

regular-file copy-up 期间可能同时出现：

- lower source 的 read cache；
- workdir temporary 对应的 upper write cache。

这不是 Overlay 对外暴露的两个 authority，而是同一个 winner 在 underlying
层完成“读 lower、写 temporary”的内部过程。publication 前，temporary cache
不能被新的 Overlay lookup/open 使用；publication 后，新的操作只使用 upper
cache。已有 lower read-only view 仍只使用它原本 pinned 的 lower cache。

如果 Asterinas VFS 的 page-cache/private-state 接口需要在 authority transition
时重新绑定，必须把这作为一次受保护的 projection/publication 操作处理，而
不是复制一个 Overlay page-cache backend。具体 VFS API 留给 Stage F，不在本稿
冻结。

## 42. 第四步的实际句柄选择

trigger 完成后，file 和 directory 路径分别选择实际对象：

| 路径 | 第 4 步的结果 | 第 5 步的 owner |
| --- | --- | --- |
| lower read-only regular file | pinned lower real-file handle | `file_io_page_cache` 转发 read/seek |
| upper-authoritative regular file | pinned upper real-file handle | `file_io_page_cache` 转发读写和其它 file op |
| writable file request | trigger 后的 upper real-file handle | upper-only write/mmap/fallocate/splice |
| read-only merged directory | `DirectoryView`、ReaddirIndex 和必要 source views | `merged_readdir_cache` |
| create/mkdir/unlink/rmdir/rename | upper parent/target directory handle，以及 mutation plan | `directory_mutations_whiteouts` |
| lower directory rename（基础范围） | 无 upper handle，返回 `EXDEV` | directory mutation module |

因此，“获得实际句柄”对 regular file 通常是一个 real-file handle；对 merged
directory 则是一个 Overlay directory projection；对 directory mutation 才是
upper real-directory handle。三者不能用同一个笼统的 `FileHandle` 概念掩盖
不同的 authority 和 visibility 规则。

## 43. Copy-up 对 B/C-2、B/C-3 的内部影响

copy-up 对 B/C-2 和 B/C-3 的影响必须区分“实际 authority 改变”和“可见
namespace 改变”。前者不一定是目录 mutation，后者才需要更新目录项集合
和 cookie。

### 43.1 B/C-2：同一个 logical object 换 authority

regular file copy-up 后，B/C-2 的主要变化是：

```text
OverlayInode / OverlayObjectState
    published authority: lower -> upper
```

原有 `(parent, name)` binding、OverlayInode identity 和 logical object 不被
替换。BindingCache 更新的是 positive binding 的 real authority/provenance，
不是新建第二个 inode，也不是重新做一次 name lookup。origin identity 继续
说明 upper object 来自哪个 lower source。

在 copy-up 尚未提交时，lower 仍是 semantic authority。已有的 lower read-only
handle 可以继续完成；新的 path lookup 不应从 workdir temporary 创建竞争
binding。只要它参与相同的 parent lock 事务，就会等待 copy-up 的 parent locks，
并在锁释放后直接观察到提交前 lower 或提交后 upper。

### 43.2 B/C-2.2：parent locks 解决普通 lookup race

对普通 non-reentrant lookup，copy-up winner 可以在完成转换期间持续持有：

- upper parent lock；
- 被复制对象所在的 lower source parent lock；
- 多 lower 场景下所有会影响本次 visibility reduction 的相关 parent lock。

并将以下步骤放在释放这些 parent locks 之前：

```text
temporary fill
    -> physical upper publication
    -> OverlayObjectState/binding authority publication
    -> release parent locks
```

这样新的 lookup 不需要频繁检查 `CopyUpInProgress` 或 `ready`：它要么在
锁前看到旧的 lower authority，要么在锁后看到完整的 upper authority。已有的
pinned lower handle 不受影响。

file-I/O copy-up 没有 caller-held Overlay `DIR` 时，可以依靠 real parent locks
完成这种 authority-only transition，但不能先取得 real parent locks、再回头
获取 Overlay `DIR`。如果操作还会改变 namespace（create、unlink、rmdir、
whiteout、opaque 或 rename），则必须由 directory mutation 先建立 parent
`DIR`，再按 `DIR -> CUL -> ...` 进入对应 parent locks。

该规则要求 underlying parent locks 的获取顺序、重入行为和多-layer 覆盖范围
能够被证明。对于可能重入 Overlay 的 underlying filesystem，仍需走明确的
lock-neutral/retry 边界；这不改变普通 non-reentrant lookup 的默认模型。

### 43.3 B/C-2.4：xino 下的 identity update

copy-up 会改变 underlying real inode，因此需要把 xino identity projection
纳入 authority publication。但这里要区分：

```text
lower raw ino       -> upper raw ino       // 可能改变
Overlay st_ino      -> 应保持 xino 语义下的稳定
directory d_ino     -> 需要重新计算/确认
```

启用 xino 时不能简单地把 upper raw inode 直接写成新的 Overlay identity。
应在 publication 中更新 real-object provenance、layer fsid、xino eligibility
以及 overflow/fallback 事实，并保证 `OverlayInode` 的 `st_ino` 与目录项
`d_ino` 仍符合 xino 的稳定性和一致性要求。对于能够使用 copy-up origin 的
对象，应继续以 origin 维持跨 copy-up 的稳定投影；无法满足 xino 条件时，
必须显式进入对应 fallback，而不是静默产生一个不一致的数字。

因此 B/C-2 的更新不是“创建新的 inode”，而是更新同一 logical inode 的
real-identity mapping。这个更新必须与 upper authority publication 一起完成，
不能留下 authority 已经切换但 identity 仍使用旧 projection 的窗口。

### 43.4 B/C-3：authority-only copy-up 不重新编号目录

如果 lower file 原本已经以该名字可见，write-open 造成的 file copy-up 通常
不改变目录的可见名字集合：

- 不插入新的 ReaddirIndex entry；
- 不删除原 entry；
- 不重新分配 cookie；
- 不替换 OverlayInode；
- 只更新该 entry 所引用的 identity/provenance，或标记其需要 revalidation。

如果 ReaddirIndex 保存的是稳定 Overlay identity，则 name、cookie、inode
slot 都保持不变；如果还保存 raw real ino/layer provenance，则在同一发布
边界中更新这些派生事实。

directory promotion 也不复制 lower children。parent 中的目录 entry 通常
保持原来的 name、identity 和 cookie；只有 upper/lower merge 输入、opaque
barrier 或可见 children 集合发生变化时，才更新或标记该目录的 ReaddirIndex。

反之，create、unlink、rmdir、whiteout、opaque 和 rename 属于 namespace-
visible mutation，必须在 parent `DIR` transaction 内更新 BindingCache、
barrier state 和 ReaddirIndex。unlink/rmdir lower target 的写入对象是
whiteout/barrier，而不是先 copy-up 被删除的 target。

### 43.5 两个模块的交接摘要

```text
copy-up
    -> B/C-2: 更新同一 logical object 的 actual authority
    -> B/C-2: 更新 xino/identity projection
    -> B/C-3:
         可见 namespace 不变 -> 保留 binding/cookie
         可见 namespace 改变 -> 更新或重建 ReaddirIndex
```

## 44. 锁、BIO 和 callback：放在完整流程之后理解

完整请求路径中的锁只服务于 authority transition，不改变第 1 至第 5 步的
语义顺序。继续遵循接受的全局拓扑：

```text
DIR -> CUL -> INODE -> WL -> UPPER
```

本阶段的实际边界是：

- object-kind/intent classification 尽量 lock-neutral；
- directory mutation 可以带着 parent `DIR` 进入 trigger，file I/O、mmap 和
  page-cache callback 不获取也不接受 caller-held `DIR`；
- copy-up coordination 使用 `CUL`；多个同级 object 按接受的
  `Arc::as_ptr()` 顺序，不递归取得同一实例；
- object authority/publication 需要时使用 `INODE`；upper real-file access
  使用 `UPPER`；本阶段不使用 `WL`；
- copy-up winner 从取得 `CUL` 起持续保持 coordination ownership，直到
  semantic publication 或失败清理结束；不在正常阶段之间反复检查 ready；
- waiter 等待 copy-up 或 callback 时不得持有它可能重新取得的 Overlay lock；
- 普通、已证明非重入的 sleep-capable underlying BIO 可以在允许 sleep 的
  mutex domain 中执行；spin lock 内禁止 BIO、sleep 或 yielding lock；
- 等待、释放锁或 callback 返回后，重新检查 object authority、operation
  intent、mount lifetime、source identity、upper target 和 file view pin。

“lock-neutral”只适用于已证明可能重入或反向取得 Overlay lock 的 callback
边界，不是普通 per-layer I/O 的默认模式。逻辑 directory transaction 也不
因为普通 BIO 而反复释放再取得 parent `DIR`。

## 45. 失败、清理和发布不变量

### 45.1 Publication 前

preflight、ancestor preparation、lower read、temporary write、metadata/xattr/
origin transfer、data copy、durability 或 atomic publication 失败时：

- lower 仍是 Overlay semantic authority；
- temporary/intermediate upper object 不可见；
- winner 清理或记录明确的 workdir cleanup obligation；
- waiter 被唤醒后重新观察并决定 retry 或返回 error；
- 不把 partial transfer 当作 short successful I/O。

### 45.2 Physical publication 后

physical upper operation 已经成功但 Overlay semantic publication 未完成时，
不承诺一般 rollback。必须先验证 upper object 的 type、origin、metadata、
target binding 和所需目录索引状态；完成验证后才能发布 upper authority。
验证失败进入 reconcile/error 状态，不能让 lookup/open/readdir 猜测 raw upper
object 的意义。

### 45.3 最小不变量

1. 写请求先触发 authority decision，再取得写用实际句柄。
2. 一个 logical object 同时只有一个 live copy-up winner。
3. lower file view、workdir temporary、upper real-file handle 和 page cache
   的生命周期不会互相冒充。
4. file copy-up 复制 regular-file data；directory promotion 不复制 children。
5. unlink/rmdir lower target 使用 whiteout/barrier 语义，不复制被删除 target。
6. lower/merged directory rename 在基础范围内不隐式启用 redirect，返回
   `EXDEV`。
7. Overlay 不拥有第二个 page cache；新的 writable operation 只能使用 upper
   real inode 的 page cache。
8. physical publication 和 semantic publication 不能被报告成两个独立成功；
   incomplete 状态必须可观察为 retry/reconcile/error。

## 46. 跨模块交接

| 交接方 | B/C-4 提供 | B/C-4 不接管 |
| --- | --- | --- |
| B/C-1 mount/layers | 消费 upper/workdir lifetime、read-only、credential 和 durability policy | 不重新解析 mount options 或重新声明 upper/workdir exclusivity |
| B/C-2 projection/identity/lookup | 把 copy-up 后的同一 logical object 切换到 upper authority，保持 Overlay identity 连续 | 不创建第二个 identity map，不做 `ID -> name` |
| B/C-3 merged readdir | directory promotion 或 upper publication 后更新 binding/identity/ReaddirIndex；同名 entry 不因 copy-up 重新编号 | 不拥有目录合并、cookie、whiteout 或 opaque authority |
| B/C-5 metadata/permission/xattr | 提供 source metadata、credential check 和 eligible xattr policy 的约束 | 不重新定义权限模型，不复制 overlay-private xattrs 为普通 metadata |
| B/C-6 directory mutation/whiteout | 为 parent preparation、upper directory handle 和 copy-up trigger 提供结果 | 不拥有 create/unlink/rmdir/rename/whiteout 的 namespace transaction |
| B/C-7 advanced identity/data | 保留 metacopy/verity/export/index 等 authority-transition seam | 基础路径不实现 metadata-only copy-up 或第二个 data owner |
| B/C-8 reconciliation | 暴露 physical/semantic publication、file-view、cache 和 directory-index 的交接点 | 不提前宣称所有跨模块冲突已解决 |

## 47. 外部验证映射（仅 xfstests）

本节是静态的 many-to-many 映射，不表示本轮已运行测试，也不把黑盒结果当作
内部锁或 page-cache identity 的证明。

| 范围 | upstream case | 可观察结果 | 限制 |
| --- | --- | --- | --- |
| `P1-02` | `overlay/006` | lower-backed mutation 穿过 trigger，产生预期 upper/whiteout 结果 | 与 directory mutation/whiteout 组合 |
| `P1-03`, `P1-34` | `overlay/023` | upper parent/ancestor preparation 和 workdir cleanup | 与 workdir/ACL cleanup 组合 |
| `P1-04` | `overlay/009`, `013`, `014`, `018`, `024`, `025`, `026`, `027`, `028`, `033`, `037` | data staging、publication、upper visibility | 与 xattr、multi-lower、setattr、hardlink、rename 或 readdir 组合 |
| `P1-05` | `overlay/008`, `015`, `016`, `025` | owner/mode/timestamp/permission preservation | 与 whiteout、SGID 或 setattr 组合 |
| `P1-06` | `overlay/009`, `014` | eligible xattr/ACL transfer | 不证明所有失败清理分支 |
| `P1-07` | `overlay/024` | lower origin identity 保存 | 不覆盖后续 index/export 消费者 |
| `P1-28` copy-up side | `overlay/018`, `overlay/028` | lower-backed hardlink source 先取得 upper authority，再由 namespace mutation 完成 upper link | actual link publication 仍由 B/C-6 负责 |
| `P1-32` object-kind side | `overlay/026` | symlink read 使用正确的 lower/upper symlink object，不复制 referent | permission check 仍由 B/C-5 的公共管线覆盖 |
| `P2-07` | `overlay/018`, `overlay/044` | lower hardlink copy-up、upper inode reuse 和 visible `nlink` 观察保持一致 | index-enabled 与 no-index 行为需按选定范围区分 |
| `P1-08`, `P1-10` | `overlay/029`, `039` | lower read/open delegation 与 mapping/data path | 组合观察，不能证明所有 handle lifetime |
| `P1-12` | `overlay/039` | lower read-only mapping divergence 与 writable mapping upper requirement | 不证明内部 single-cache identity |
| `P1-13` | `overlay/040` | file synchronization delegation | 不隔离每种 fsync mode |
| `P1-01`, `P1-09`, `P1-11`, `P1-14`, `P1-15`, `P1-37` 内部语义 | 无 isolating case | 显式记录 upstream coverage gap | 不得用 ktest、local fixture 或 filesystem-local test 补齐 |
| 条件 `P2-08`, `P2-09`, `P2-10` | 暂无 isolating case；`029/039/040` 只能提供相邻观察 | 等 Stage D 决定是否纳入 | 不将相邻行为升级为 feature coverage |

若未来进入 Checker 阶段，Creator-synced Checker 必须镜像 exact Creator
micro set；meso integration 另行安排。实际运行还必须保存每个选定 test 的
result file、guest log 和 `PASS`/`FAIL`/`NOTRUN` 结果。当前仍是设计阶段，不
运行构建/测试，不创建或修改 ktest/xfstests surface。
