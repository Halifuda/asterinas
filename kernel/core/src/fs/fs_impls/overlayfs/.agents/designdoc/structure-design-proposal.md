<!-- SPDX-License-Identifier: MPL-2.0 -->

# Overlayfs 代码结构设计提案（->重新实现）

## 动机

legacy overlayfs 是单文件实现，扩展性和可维护性差，存在并发问题，且基本功能不完整。新 overlayfs 实现的功能已经完备，但评审反馈集中在结构上：

- “无法隐藏复杂性”：两个中心类型 `OverlayFs`/`OverlayInode` 的 impl 块散布在约 15 个文件中，读者无法以类型为锚阅读；
- 顶层模块平铺：5 个顶层模块平铺，读者无法得到有序的代码审阅流程预期；
- 存在部分过度抽象和过度设计。

本文提出一套结构设计：从 overlayfs 的领域概念出发推导代码结构，使模块可以逐层向下展开阅读。

## 设计

本节按 overlayfs 的领域概念自上而下给出代码结构的规格说明。每一条目定义该层/模块的职责、归属、关键类型与依赖方向。

### 1. 顶层入口

overlayfs 将 upper 可写层与 lower 只读层合并为一个 VFS 可见命名空间。代码顶层由以下入口组成：

- `mod.rs`：模块声明与 `init`。
- `fs_type.rs`：`OverlayFsType`，VFS 注册类型。
- `fs/`：挂载模块，表示一次 mount 拥有的状态与构造流程。
- `inode/`：逻辑对象模块，表示 overlay 暴露给 VFS 的 `OverlayInode` 及其行为族。
- `real.rs`
- `layer.rs`

依赖方向上，`fs/`、`inode/` 会依赖 `real.rs` 与 `layer.rs`；这两个基础文件放在第 5 节讲解。

### 2. 挂载对象 `OverlayFs`

`fs/mod.rs` 定义 `OverlayFs`，作为一次挂载拥有的全部状态：

```rust
pub struct OverlayFs {
    fs_event_stats: FsEventSubscriberStats,
    inodes: InodeCache,
    identity: IdentityPolicy,
    layer_stack: LayerStack,
    policy: MountPolicy,
    self_weak: Weak<OverlayFs>,
    upper_workdir_pair: Option<UpperWorkdirInuse>,
    whiteout_cache: Mutex<WhiteoutCache>,
    _anon_device_id: AnonDeviceId,
}
```

字段说明：(可写为注释)（此处可添加overlayfs语义说明）

- `inodes`：inode 身份复用缓存，保证同一真实对象映射到同一 `OverlayInode`。
- `identity`：基于 `st_dev`/`st_ino` 向 VFS 暴露 dev 与 ino 的策略。
- `layer_stack`：overlay 的层栈；`LayerStack` 定义见第 5 节。
- `policy`：运行期只读的挂载策略。
- `upper_workdir_pair`：upper/workdir 的排他持有；`Some` 仅对可写挂载存在。（介绍upper/lower后介绍与底层并发操作时的假设）
- `whiteout_cache`：whiteout 单槽共享缓存。

职责：代表一次 mount 的全局状态，并提供 `FileSystem` 实现与访问器契约。

### 3. 挂载构造流程 `fs/mount/`

`fs/mount/` 为一次性构造流程，按 mount 参数处理顺序组织：

1. `options.rs`：解析 `MountOptions`。当前仅识别已实现的功能选项：`lowerdir`、`upperdir`、`workdir`、`uuid`、`xino`、`default_permissions`。
2. `layer_parts.rs`：解析 upper/lower/workdir root path，执行实例稳定性校验，组装 `LayerParts` 并装配 `LayerStack`（定义见第 5 节）。
3. `inuse.rs`：排他认领 upper/workdir 并准备 workdir。`UpperWorkdirInuse` 持有 upper/workdir 的 `InuseGuard`、统一身份 `Uuid` 与 workdir workspace；排他性通过 VFS `OverlayInuseSlot` 保证，防止多个 overlay 共用同一 upper/workdir。
4. `capabilities.rs`：测量 upper 能力，包括 d_type 支持、overlay 私有 xattr 支持等；能力不足时决定降级或拒绝。
5. `mod.rs`：编排以上步骤。

产物：`fs/policy.rs` 中的运行期只读策略对象 `MountPolicy`。

### 4. 逻辑对象 `OverlayInode`

`inode/mod.rs` 定义 `OverlayInode`，是 overlay 暴露给 VFS 的逻辑对象载体，实现 `Inode` 与 `FileOps`。

```rust
struct OverlayInode {
    copyup_transition: Mutex<CopyUpTransition>,
    extension: Extension,
    fs: Weak<OverlayFs>,
    lock: Mutex<Option<ReaddirIndex>>, (普通文件是否需要锁)
    lowers: Vec<RealObject>,
    object_id: ObjectId,
    upper: Once<RealObject>,
}
```

字段说明：

- `lowers`、`upper`：对象引用的底层 fs 真实对象（`RealObject` 定义见第 5 节）。`upper: Once<RealObject>` 表示 copy-up 是 lower→upper 的单向、至多一次发布；读路径无锁，发布时原子写入。
- `copyup_transition`：copy-up 协调状态（CUL）。copy-up 是多步、可阻塞 IO 的协议，每个对象至多执行一次；`Once` 只能表达“已完成/未开始”，无法表达“进行中”，因此需要一把可睡眠的每对象锁做 winner/waiter 仲裁：winner 持锁跑完整个 copy-up，waiter 睡到锁后用 `upper.get()` 复查并退出。（detail）
- `extension`：VFS 提供的每 inode 扩展状态（事件发布/锁上下文），由 VFS 自己同步，在 OverlayFs 中主要负责处理 `OverlayInuseSlot`。
- `lock: Mutex<Option<ReaddirIndex>>`：每 inode 唯一的事务锁。目录时 payload 为 `ReaddirIndex`，作为命名空间事务域；mutation、lookup、readdir、索引维护都在域内。非目录时 payload 为 `None`，锁仅为纯串行令牌。
- `object_id`：预计算的对外 `st_dev`/`st_ino`。

锁序：

- 跨层边只有一条 `CUL → lock`：copy-up 的 commit 段短暂持有 publication_parent 的 `lock`，使发布与命名空间变更互斥。
- rename 取两个父目录的 `lock` 按地址序（对齐 `lock_rename`）。
- 禁止规则：由于不存在可重入锁，持 `lock` 时永不等待 CUL，copy-up 前置到取 `lock` 之前（`check_permission` / 源升格先执行，再取锁），故无环。

### 5. 公共基础类型：`real.rs` 与 `layer.rs`

`real.rs` 与 `layer.rs` 位于最底层；`fs/`、`inode/` 依赖它们，无反向依赖。

#### 5.1 `real.rs` 的内容

`real.rs` 描述 overlay 所引用的底层文件系统对象：

```rust
pub struct RealObject {
    layer_index: usize,
    real_inode: Arc<dyn Inode>,
    real_path: Option<RealPath>,
    fsid: u64,
    container_dev_id: DeviceId,
}

pub struct RealPath {
    mount: Weak<Mount>,
    dentry: Arc<Dentry>,
}

pub struct RealObjectKey {
    fsid: u64,
    real_ino: u64,
}
```

- `RealObject`：一个被 overlay 引用的真实文件系统对象；overlay 会为其记录所在层的序号、真实 inode、dentry 锚定的 `RealPath`、fsid 与设备号。
- - `real_path` 采用 `Option<RealPath>`，因为 `RealObject` 同时服务“仅需身份比较”与“需要路径操作”两类场景：身份类场景（如 ".."）只需要 `real_inode`、`fsid` 等身份字段，路径类场景才需要 entry 锚定的 `RealPath`。`real_inode` 是身份与操作委托的基础，在无路径时仍可用。
- `RealPath`：真实对象的 dentry 锚定路径，可升级为 `Path`。它只保存 `Weak<Mount>` 与 `Arc<Dentry>`，可通过 `Dentry`/`Path` 取得实际的底层 inode。
- `RealObjectKey`：由真实对象身份（fsid + real inode）构成的主索引，每个底层的 inode 只有一个 key。

#### 5.2 `layer.rs` 的内容

`layer.rs` 描述 overlay 的 fs 层模型：

```rust
pub struct Layer {
    root_path: RealPath,
    fs: Arc<dyn FileSystem>,
    fsid: u64,
    container_dev_id: DeviceId,
}

pub struct LayerStack {
    upper: Option<Layer>,
    lowers: Vec<Layer>,
}

pub struct RealObjectStack {
    upper: Option<RealObject>,
    lowers: Vec<RealObject>,
}
```

- `LayerStack`：overlay 的层栈结构。`LayerStack` 表示一个可写 upper 与若干只读 lower 的层次集合；`Layer` 表示其中一层。
- `Layer`：描述静态层根，持有确定存在的 root `RealPath` 与文件系统身份。
- - `fsid`：每次挂载根据 layer stack 顺序动态分配的 id，多次挂载间可能改变；处于同一个底层 fs 的所有 layer 共享同一个 fsid。
- - `container_dev_id`：底层 fs 实际所处的设备的 id，多次挂载不改变。
- - lookup 过程中基于 `RealPath` 与上述各 id 派生 `RealObject`。
- `RealObjectStack`：一个 overlay 对象背后的真实对象组合，包含可选的 upper `RealObject` 与 lower `RealObject` 列表。

### 6. 名字解析 `inode/lookup.rs`

名字解析规则：upper 优先逐层向下；命中同名 whiteout、opaque 父目录或同名非目录文件即停。

- `Lookup` / `NegativeLookup`：lookup 的返回形态：

```rust
pub enum Lookup {
    Positive(Arc<OverlayInode>),
    Negative(NegativeLookup),
}

pub enum NegativeLookup {
    Absent,
    HiddenByWhiteout,
    HiddenByOpaque,
}
```

结构流程：

1. `lookup_in_layers` 扫描各层；命中 positive 时内部构造 `RealObjectStack`。
2. `project_inode` 将该 `RealObjectStack` 转换为 `Arc<OverlayInode>`。这一步利用 `RealObjectStack` 得到 `RealObjectKey` 并通过 inode cache 复用 `OverlayInode`，若 inode cache 中无对应条目则创建 inode 并插入 cache。
3. 所有层均未命中或命中 whiteout、opaque 时创建 `NegativeLookup`。
4. 最终统一返回 `Lookup`。

### 7. inode 身份复用 `inode/inode_cache.rs`

同一真实对象经任何名字解析必须得到同一个 `OverlayInode`，否则真实对象组合、追加写锁、copy-up 协调会分裂。本模块实现 inode cache 的操作逻辑。`InodeCache` 以 `RealObjectKey`（定义见第 5 节）为键、`Weak<OverlayInode>` 为值。通过 get-or-create 保证同一真实对象只构造一个 `OverlayInode`。`InodeCache` 有一内部锁，只在缓存读写时获取，随后释放，内部不再取其它锁。

### 8. 用户可见身份 `inode/identity.rs`

`IdentityPolicy` 负责将真实对象映射为 overlay 对外可见的 `st_dev`/`st_ino`。它根据挂载时的 `XinoMode` 选择不同的策略。因此不同挂载的 `ObjectId` 规则可以不同。

本节定义：

- `IdentityPolicy`：身份投影策略，包含 xino 编码、fallback ino 分配等。
- `ObjectId`：一个 overlay 对象的对外 `st_dev`/`st_ino`，在 inode 创建时按 `IdentityPolicy` 计算并保存在 `OverlayInode.object_id`。
- `LowerIdOrigin`：copy-up 前 lower source 的持久化身份记录，用于在支持 origin 的场景下维持身份稳定。

至此定义全 overlayfs 所用的两个对象索引：

- `ObjectId`：overlay 对外身份，位于 `inode/identity.rs`。
- `RealObjectKey`：overlay 内部真实身份，位于 `real.rs`（见第 5 节）。

### 9. 目录枚举 `inode/readdir.rs`

合并目录的可见名字来自 upper + lowers 的合并视图，不能每次 getdents 都重新扫描全层；`ReaddirIndex` 将可见名字排列为单调 cookie 序列，提供稳定、可续的枚举顺序，并在 mutation 后失效、经 lookup 重建。`..` 的身份也在该模块解析。目录文件的 `lock` payload 即 `ReaddirIndex`，readdir 与索引维护在该锁域内进行。本模块包含 `ReaddirIndex` 提供单调且稳定的 dentry cookie 序列的算法。外部模块如 inode 不需关心实现细节。

### 10. 写路径与 copy-up `inode/copyup/`

lower 只读，写之前必须将对象复制到 upper。`inode/copyup/mod.rs` 定义完整流程：

1. 仲裁：winner/waiter + 祖先链。copy-up 是多步、可阻塞 IO 的操作，同一对象可能被多个任务并发触发；winner 持 CUL 执行完整升格，waiter 等待后通过 `Once` 复查结果。
2. 制备：按类型搬移数据/metadata/xattr 到 workdir 暂存。
3. 发布：rename 原子换入，并通过 `Once` 发布 upper。commit 段短暂持有 publication_parent 的 `lock`。

workdir 临时对象创建、维护、删除的具体操作由 `inode/copyup/workdir.rs` 实现并提供接口。

### 11. 命名空间变更 `inode/dir/`

`inode/dir/` 包含所有“对父目录命名空间做变更”的操作。这些操作共享同一套流程，因此放在同一子树：

- `dir/mod.rs`：共用操作流程（父目录事务锁 → 准入 → Stale 判别 → 物理操作 → 索引维护）。
- `create.rs` / `link.rs` / `remove.rs` / `rename.rs`：四种命名空间变更。
- `whiteout.rs`：whiteout 表示（char 0:0 或 xattr）、发布与清扫。删除 lower 可见名字时不能修改 lower，改为在 upper 发布遮蔽物；whiteout 是这一机制的承载模块，与命名空间变更同属 `dir/`。
- - `WhiteoutCache` 保存一个可复用的 workdir whiteout temp handle 和 `can_share_by_link` 标志，供不同目录的 remove/rename 复用，避免每次发布都重建 whiteout。`WhiteoutCache` 的锁是叶子锁，它只覆盖 `WhiteoutCache` 的读写，内部不再获取其它锁。

### 12. 权限、属性、xattr

#### 12.1 权限 `inode/permission.rs`

权限检查为两段式：

1. OverlayFs 检查挂载权限
2. 写操作触发 copy-up
3. 在真实对象上检查真实权限

本模块实现上述权限检查与 copy-up 触发，并提供权限检查入口。

#### 12.2 属性写 `inode/metadata.rs`

属性写（mode/owner/group/time）经过权限管线后转发到真实对象。本模块未来预计承担 metacopy feature。

#### 12.3 xattr `inode/xattr.rs`

xattr 操作包括对 VFS 暴露的 get/set/list/remove，以及 overlay 私有 xattr （如 whiteout、opaque） 的分类、权限检查与读写。

### 13. 数据读写 `inode/data.rs`

选择由 `RealObjectStack`（定义见第 5 节）推导的当前权威真实对象进行转发：

- 读 lower 时带 `O_NOATIME`。
- `O_APPEND` 在每 inode 事务锁（`lock`）下串行。
- 可写 open 先 copy-up。

## 文件结构

```text
overlayfs/
├── mod.rs                    — 模块声明 + init
├── fs_type.rs                — OverlayFsType：VFS 注册类型
├── layer.rs                  — Layer / LayerStack / RealObjectStack
├── real.rs                   — RealObject / RealPath / RealObjectKey
│        
├── fs/                       — 挂载模块（一次 mount 拥有的全部）
│   ├── mod.rs                — OverlayFs + FileSystem 实现
│   ├── policy.rs             — 挂载后运行时可读 MountPolicy
│   └── mount/                — 构造流程
│       ├── mod.rs            — 构造编排
│       ├── options.rs        — MountOptions
│       ├── layer_parts.rs    — root path 解析 / LayerParts / LayerStack::assemble
│       ├── inuse.rs          — Uuid / UpperWorkdirInuse / InuseGuard
│       └── capabilities.rs   — UpperFilesystemCapabilities / DTypeProbeVisitor
│
└── inode/                    — 逻辑对象
    ├── mod.rs                — OverlayInode + Inode·FileOps 实现
    ├── inode_cache.rs        — InodeCache / InodeCacheEntry
    ├── lookup.rs             — Lookup / NegativeLookup + 层扫描 + 构造编排
    ├── identity.rs           — ObjectId / IdentityPolicy / LowerLayerIdentity / LowerIdOrigin
    ├── readdir.rs            — ReaddirIndex 系 + 枚举服务 + `..` 身份
    ├── copyup/
    │   ├── mod.rs            — CopyUpTransition / CopyUpPhase + 仲裁/制备/发布
    │   └── workdir.rs        — WorkdirTemp / WorkdirTempRequest
    ├── dir/
    │   ├── mod.rs            — 族形状（事务锁 / 索引维护 / impure 刷新）
    │   ├── create.rs / link.rs / remove.rs(RemoveKind) / rename.rs
    │   └── whiteout.rs       — WhiteoutCache（+私有的 Handle/Representation）+ 检测/发布/清扫
    ├── data.rs               — 数据通路委托规则
    ├── permission.rs         — AccessType + 两段式准入管线 + 凭证探测
    ├── metadata.rs           — 属性写
    └── xattr.rs              — xattr 操作与 overlay 私有记录
```
