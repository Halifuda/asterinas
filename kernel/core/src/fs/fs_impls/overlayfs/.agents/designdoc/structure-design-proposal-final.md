<!-- SPDX-License-Identifier: MPL-2.0 -->

# 介绍 `overlayfs` 的结构性重构设计

## 动机

legacy overlayfs 是单文件实现，扩展性和可维护性差，存在并发问题，且基本功能不完整。本 issue 从 overlayfs 的领域概念出发，提出一套重新实现设计，并明确 overlayfs 的持有与引用模型。

### Legacy 实现的主要缺憾

- **单文件 monolith**：所有功能都挤在一个 `legacy_fs.rs` 文件中，读者很难按类型定位行为，也难以扩展。
- **身份与并发不安全**：同一个底层文件被多次访问时会生成多个不同的 overlay inode，状态无法共享；并发写者可能重复执行同一操作。
- **工作目录行为不符合 Linux**：复制提升不使用隔离的工作目录做原子替换，崩溃时可能留下半成品；多个 overlay 可能共用同一个工作目录，互相干扰。
- **功能与正确性不足**：只读 overlay、跨目录重命名、同步、超级块信息和权限检查等能力缺失；目录枚举在大目录下低效且续读位置不稳定，对外文件标识（dev/ino）也不稳定。

### 重新实现的目标

本 issue 提出重新实现 overlayfs，目标是让代码以核心类型为锚、按领域概念组织，并修复 legacy 在身份复用、并发、工作目录、枚举稳定性等方面的问题。设计上强调清晰的持有关系和引用模型，使每个机制都有明确的归属与阅读路径。

## 设计
### 1. 总体结构

代码顶层由以下入口组成：

- `mod.rs`：模块声明与 `init`。
- `fs_type.rs`：`OverlayFsType`，VFS 注册类型。
- `fs/`：挂载模块，表示一次 mount 拥有的状态与构造流程。
- `inode/`：逻辑对象模块，表示 overlay 暴露给 VFS 的 `OverlayInode` 及其行为族。
- `real.rs`：真实对象引用模型。
- `layer.rs`：层模型与瞬时真实对象栈。

依赖方向上，`fs/` 与 `inode/` 都依赖 `real.rs` 与 `layer.rs`；`fs/` 还依赖 `inode/` 中的缓存与 whiteout 类型。这两个基础文件在最底层。

### 2. 挂载对象 `OverlayFs`

`fs/mod.rs` 定义 `OverlayFs`，作为一次挂载拥有的全部状态：

```rust
pub struct OverlayFs {
    /// 层栈：由至多一个可写层（upper）与一个或多个只读层（lower）组成。
    layer_stack: LayerStack,
    /// 运行期只读挂载策略。
    policy: MountPolicy,
    /// 对外 dev/ino 身份换算策略。
    identity: IdentityPolicy,
    /// upper/workdir 的排他持有；`Some` 仅对可写挂载存在。
    upper_workdir_pair: Option<UpperWorkdirInuse>,
    /// whiteout 单槽共享缓存。
    whiteout_cache: Mutex<WhiteoutCache>,
    /// inode 身份复用缓存。
    inodes: InodeCache,
    /// VFS 事件统计。
    fs_event_stats: FsEventSubscriberStats,
    /// 用于构造 root inode 的弱自引用。
    self_weak: Weak<OverlayFs>,
    /// overlay 的 `AnonDeviceId` RAII guard。
    _anon_device_id: AnonDeviceId,
}
```

这里先简要说明几个字段对应的 overlay 概念：

- `layer_stack`：overlayfs 在挂载时把多个底层目录按顺序组装成层栈，并维持一个 upper-first 的合并视图；详见 §5 层模型。
- `policy`：保存运行期挂载策略，例如只读模式和权限设置，并决定是否允许写入和 copy-up。
- `identity`：持有 dev/ino 身份换算策略，用于计算每个 overlay 对象的对外可见身份；详见 §10。
- `upper_workdir_pair`：持有可写挂载的 upper/workdir 排他认领与 workdir 暂存资源；详见 §3 与 §9。
- `whiteout_cache`：与隐藏 lower 名字的 whiteout 机制相关的挂载级共享缓存；这里先只说明它服务于目录命名空间变更，详见 §12。
- `inodes`：inode 身份复用缓存；详见 §8。

字段顺序采用“核心不可变状态 → 同步状态 → 缓存/资源/弱引用”，便于从上到下阅读对象本质。

### 3. 挂载构造流程 `fs/mount/`

`fs/mount/` 为一次性构造流程：

1. `options.rs`：解析 `MountOptions`。
2. `layer_parts.rs`：解析 upper/lower/workdir root path，校验并组装 `LayerStack`。这里直接解析出 `Arc<Mount>` 与 `Arc<Dentry>` 作为 `Layer` 的输入。校验内容包括：layer root 不能相同、不能互为祖先/后代、不能跨越挂载边界；workdir 不能与 lower roots 冲突。
3. `inuse.rs`：排他认领 upper/workdir 并准备 workdir。排他之所以必要，是因为多个 overlay 共享同一个 upper/workdir 时，会互相覆盖 whiteout、临时对象和目录状态，损坏彼此的数据。
4. `capabilities.rs`：测量 upper 能力，例如 `d_type` 支持与 overlay 私有 xattr 支持，并据此决定 whiteout 表示与其他挂载策略。
5. `mod.rs`：编排以上步骤。

### 4. 逻辑对象 `OverlayInode`

`OverlayInode` 是 overlay 暴露给 VFS 的逻辑对象载体：

```rust
pub struct OverlayInode {
    /// 所属 overlay 挂载。
    fs: Weak<OverlayFs>,
    /// 不可变的 lower 真实对象栈，topmost first；lower 是只读层中的真实对象。
    lowers: Vec<RealObject>,
    /// upper 真实对象；upper 是可写层中的真实对象，copy-up 时至多发布一次。
    upper: Once<RealObject>,
    /// 预计算的对外 `st_dev` / `st_ino`。
    object_id: ObjectId,
    /// 每 inode 唯一事务锁；目录时携带 `ReaddirIndex`（merged directory 的稳定枚举索引）。
    lock: Mutex<Option<ReaddirIndex>>,
    /// 逻辑 overlay 父目录；同时也是 copy-up 的发布父目录。
    parent: RwMutex<Weak<OverlayInode>>,
    /// copy-up 状态：Done 或 Outstanding(CopyUpTarget)。
    copyup: Mutex<CopyUpState>,
    /// VFS 提供的每 inode 扩展状态。
    extension: Extension,
}
```

字段说明：

- `lowers` / `upper`：lower 是只读层真实对象，upper 是可写层真实对象。同名目录在多层都存在时，overlay 会形成 merged directory（见 §11）。`upper: Once<RealObject>` 表示 copy-up 是 lower→upper 的单向、至多一次发布；读路径无锁，发布时原子写入。
- `parent`：逻辑 overlay 父目录与 copy-up 发布父目录。
- `copyup`：保存 copy-up 状态；详细语义见 §9。
- `lock`：目录事务锁与 readdir 索引域；`ReaddirIndex` 是 merged directory 的稳定枚举索引，见 §11。非目录时作为纯串行令牌。
- `object_id`：预计算的对外 dev/ino，由 `IdentityPolicy` 计算，见 §10。

### 5. 层模型 `layer.rs`

overlayfs 把若干底层目录叠成一个可见命名空间：其中至多一个可写目录作为 **upper**，其余只读目录作为 **lower**，lookup 时先看 upper，再按 lower 的 top-to-bottom 顺序向下找。`Layer` 表示其中一层：

```rust
pub struct Layer {
    /// 强持有底层 mount；`Mount` 持有 `Arc<dyn FileSystem>` 与 root dentry。
    mount: Arc<Mount>,
    /// 层根 dentry；upperdir/lowerdir 可以指向某 mount 下的子目录，因此必须显式保存。
    root_dentry: Arc<Dentry>,
    /// 每次挂载根据 layer stack 顺序动态分配的 fs id。
    fsid: u64,
    /// 底层 fs 的设备 id。
    container_dev_id: DeviceId,
}
```

`Layer` 是 overlay 对底层 mount/fs 的**单一强持有者**：只有强持有 mount，才能保证 overlay 挂载存活期间底层文件系统不会被先卸载；`RealPath`（见 §6）使用 weak mount，属于临时、可重新解析的锚点，不能承担 keep-alive 责任。

`root_dentry` 与 `mount.root_dentry()` 不一定相同。`lowerdir` / `upperdir` 可以指向某个 mount 下的子目录，不一定等于该 mount 的根；因此 `Layer` 必须显式保存层根 dentry，不能假设层根一定是 mount root。

overlay 假设挂载期间 upper/lower 不会被绕过 overlay 直接修改；直接修改底层属于未定义行为。

`LayerStack` 表示 upper/lower 层的有序集合，其中 `lowers` 在挂载时至少有一个：

```rust
pub struct LayerStack {
    upper: Option<Layer>,
    lowers: Vec<Layer>,
}
```

瞬时真实对象栈作为 lookup/readdir 的扫描载体（见 §7）：

```rust
pub struct RealObjectStack {
    upper: Option<RealObject>,
    lowers: Vec<RealObject>,
}
```

### 6. 真实对象引用 `real.rs`

`RealPath` 是一个 weak-mount 的 dentry 锚定载体：

```rust
pub struct RealPath {
    mount: Weak<Mount>,
    dentry: Arc<Dentry>,
}

impl RealPath {
    /// 不可失败：保存的 `Arc<Dentry>` 保持 inode 存活。
    pub fn inode(&self) -> &Arc<dyn Inode> {
        self.dentry.inode()
    }

    pub fn upgrade(&self) -> Result<Path> {
        Path::new(self.mount.upgrade()?, self.dentry.clone())
    }
}
```

所有真实对象都来自已解析的底层路径，因此 `RealObject` 始终 path-backed：

```rust
pub struct RealObject {
    /// 所在层序号。
    layer_index: usize,
    /// dentry 锚定的真实路径。
    path: RealPath,
    /// 所在层对应的 fs id。
    fsid: u64,
    /// 底层 fs 的设备 id。
    container_dev_id: DeviceId,
}
```

`RealObjectKey` 是 overlay 内部标识真实对象身份的值类型，由 `fsid` 与真实 inode 号组成，用作 inode cache 的键：

```rust
pub struct RealObjectKey {
    fsid: u64,
    real_ino: u64,
}
```

### 7. 名字解析 `inode/lookup.rs`

lookup 的规则如下：

- **upper-first**：先查 upper，再按 lower top-to-bottom 顺序逐层向下。
- **first-wins**：合并结果只保留第一次出现的名字，上层优先。
- **whiteout 停止**：某一层中的 whiteout 遮蔽更下层的同名对象，停止继续向下扫描该名字。
- **opaque 目录停止**：某一层目录带 opaque 标记时，更下层的同名目录停止参与合并。
- **同名非目录停止**：同一名字在高层是非目录时，不能与低层目录合并，停止继续向下扫描。

whiteout 是某一层中的遮蔽标记，用来隐藏更下层的同名对象；opaque 目录是某一层目录上的标记，用来阻止更下层目录继续参与合并。这些标记通常由 upper 写入，但任何一层出现都会影响更下层。详细表示与发布见 §12。

`Lookup` 表示一次名字解析的结果：`Positive` 是命中的逻辑 inode，`Negative` 是未命中或被遮蔽。

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

流程：

1. `lookup_in_layers` 扫描各层，构造 `RealObjectStack`；
2. 正向命中时按命中来源构造，以 `(parent, name)` 初始化 `OverlayInode`：新 inode 的 `parent` 和 `CopyUpState` 在构造时完成初始化；upper-backed 对象为 `Done`，lower-backed 对象为 `Outstanding`；
3. 未命中或命中 whiteout/opaque 时构造 `NegativeLookup`；
4. 返回 `Lookup`。

发布目标 `(parent, name)` 在构造时记录。

### 8. inode 身份复用 `inode/inode_cache.rs`

```rust
/// 以当前可见来源身份为键的弱引用映射，指向共享的逻辑 overlay inode。缓存不会保持 overlay inode 存活。
struct InodeCache {
    entries: HashMap<RealObjectKey, Weak<OverlayInode>>,
}

impl InodeCache {
    /// 返回同一真实对象已存在的存活 overlay inode；不存在时构造并发布一个新 inode。
    fn get_or_create(
        &mut self,
        key: RealObjectKey,
        create: impl FnOnce() -> Arc<OverlayInode>,
    ) -> Arc<OverlayInode>;

    /// 在 copy-up 发布 upper 对象后，把逻辑 inode 的缓存身份从 copy-up 前的 lower key
    /// 迁移到新的 upper key。`OverlayInode` 对象本身保持不变。
    fn rekey(&mut self, old_lower: RealObjectKey, new_upper: RealObjectKey);
}
```

同一真实对象经任何名字解析必须得到同一个 `OverlayInode`，否则真实对象组合、追加写锁、copy-up 协调会分裂。`RealObjectKey` 基于当前可见来源（topmost 命中的真实对象）计算。merged directory 的完整 lower 栈保存在 `OverlayInode` 内部，所以 key 只标识当前可见来源。

### 9. 写路径与 copy-up `inode/copyup/`

```rust
/// 一个 overlay inode 的 copy-up 状态。
enum CopyUpState {
    /// 已经 upper-backed；copy-up 已完成。
    Done,
    /// 仍为 lower-backed；copy-up 尚未完成。
    Outstanding(CopyUpTarget),
}

struct CopyUpTarget {
    /// 用于在父目录中发布的目录项名字。
    name: String,
    /// 当物理 rename 已成功、但 overlay 内部状态更新失败时为 true，
    /// 因此下一次 copy-up 必须先验证 upper 目标。
    need_repair: bool,
}
```

状态定义描述了对象发布在哪里；下面的流程展示如何从加锁状态中读取该目标，并用它暂存和发布 upper 对象。

```rust
/// 获取每对象 copy-up 互斥锁。该每对象互斥锁会串行化同一对象上的并发 copy-up：胜者完成后，
/// 后来的调用者看到 `upper` 已设置就直接返回，不会重复 copy-up。
/// 调用者先检查 `upper`；guard 同时暴露记录的发布目标。
fn lock_copyup(inode: &Arc<OverlayInode>) -> Result<MutexGuard<CopyUpState>>;

/// 在 workdir 中暂存数据/metadata/xattr。
fn stage_in_workdir(inode: &Arc<OverlayInode>, target: &CopyUpTarget) -> Result<WorkdirTemp>;

/// 把暂存于 workdir 的对象原子重命名进 upper 父目录。
fn publish_by_rename(
    inode: &Arc<OverlayInode>,
    target: &CopyUpTarget,
    staged: WorkdirTemp,
) -> Result<()>;

fn copy_up(inode: &Arc<OverlayInode>) -> Result<()> {
    if inode.upper.get().is_some() {
        return Ok(());
    }

    let guard = lock_copyup(inode)?;
    let target = match &*guard {
        CopyUpState::Done => return Ok(()),
        CopyUpState::Outstanding(target) => target,
    };

    let staged = stage_in_workdir(inode, target)?;
    publish_by_rename(inode, target, staged)
}
```

基于锁的提升触发中的祖先链保证：child 只有在 parent 目录已存在于 upper 之后才会被发布。workdir 暂存避免暴露半成品：发布前崩溃只会留下一个私有 workdir 对象，而不会留下可见的 upper 条目。

当物理 rename 已经成功、但 overlay 内部状态更新失败时，会设置 `need_repair`。下一次 copy-up 会先验证 upper 目标是否与 lower 一致，然后要么继续复用它，要么报错。

### 10. 用户可见身份 `inode/identity.rs`

overlayfs 需要单独的 identity 模块，因为合并多个底层文件系统后不能简单暴露底层 `st_dev` / `st_ino`：

- 不同底层文件系统可能有冲突的 `st_dev` / `st_ino`，直接透传会造成用户态误判同一对象；
- overlay 需要根据底层能力和挂载配置选择 passthrough（直接透传底层 dev/ino）、xino 编码（对底层 ino 重新编码以避免冲突）或 fallback（回退到 overlay 分配的身份），以提供稳定且不冲突的对外身份；
- 身份在 copy-up 前后必须保持稳定：copy-up 改变物理来源，但用户看到的还是同一个 overlay 对象；
- `ObjectId` 与 `LowerIdOrigin` 是这一模块的核心实体。

`IdentityPolicy` 将真实对象映射为 overlay 对外可见的 `st_dev` / `st_ino`：

- `ObjectId`：一个 overlay 对象的对外 dev/ino；
- `LowerIdOrigin`：copy-up 前 lower source 的持久化身份记录。

### 11. 目录枚举 `inode/readdir.rs`

当同名目录在 upper 与 lower 中都存在时，overlay 呈现一个 **merged directory**：可见名字是 upper 与各 lower 的并集，upper 优先，元数据以 upper 为准。

readdir 的目标是提供稳定、可续的枚举顺序，避免每次 getdents（读取目录项的系统调用）都全量重扫。

合并目录的枚举分为以下几个阶段：

1. **按需构建索引**：目录第一次被枚举时，按 upper-first、lower top-to-bottom 的顺序扫描各真实层；每个名字只保留第一次出现；whiteout、opaque 目录和同名非目录会抑制或终止下层名字继续合并。
2. **稳定 cookie 序列**：`.` 与 `..` 使用固定 cookie，其余每个可见名字按序获得一个 cookie；所有 cookie 单调递增且永不复用；调用方以 cookie 作为 offset（续读位置）从上次位置续读。
3. **重建与失效**：命名空间变更后，如果无法证明现有顺序仍然成立，目录索引标记为需要重建，下次枚举时重新扫描。create/link 可能插入一个无法在不全量扫描的情况下证明真实位置的名字；rename 会重排名字；whiteout/opaque 变化会改变可见性，这些情况都可能触发重建。删除名字可以保留其 cookie 位置（tombstone），避免已暴露的 offset（续读位置）被重编号。

### 12. 命名空间变更 `inode/dir/`

`inode/dir/` 包含所有对父目录命名空间做变更的操作：

- `create.rs` / `link.rs` / `remove.rs` / `rename.rs`
- `whiteout.rs`：whiteout 的表示、发布与清扫

删除 lower 可见名字时不能修改 lower，而是在 upper 发布 whiteout 遮蔽物。whiteout 的物理表示根据 upper 能力选择：可以是 char device `0:0`，也可以是带 `trusted.overlay.whiteout` xattr 的零大小文件。

`WhiteoutCache` 是一个挂载级、单槽的复用池，缓存的是位于 workdir 中的私有 whiteout 临时对象，用于后续发布为 upper 中的 whiteout。它的原理是：同一个 workdir whiteout 可以通过 hard link 发布到多个 upper 目录/名字下，因此 remove/rename 不必每次重新创建 whiteout 临时对象。

使用方式：

- remove 和 rename 的 lower-backed 路径都通过同一个 `publish_whiteout` 入口发布 whiteout。
- 只有当 upper 中该名字尚不存在、且底层支持 hard link 共享时，发布成功后才会把 workdir whiteout 存回缓存。
- 如果目标已有 upper 对象、需要 rename-over 发布，或者 hard link 共享失败，则 whiteout 临时对象被消费掉，不回填缓存。hard link 共享可能因 `EMLINK`（达到 link count 上限）或 `EOPNOTSUPP`（后端不支持该操作）失败。
- `can_share_by_link` 一旦因 hard link 失败降为 false 就保持禁用；后续 whiteout 都通过消费新临时对象发布。

缓存的临时对象始终位于 workdir，属于私有对象；残留的 workdir 条目会在下次挂载准备 workdir 时清理。

### 13. 权限、属性、xattr、数据

- `inode/permission.rs`：两段式权限检查——先做 overlay 本地权限检查，需要时按需执行 copy-up，再检查底层真实对象权限；copy-up 是两段之间的动作，不是第三段权限检查。
- `inode/metadata.rs`：属性写。
- `inode/xattr.rs`：xattr 操作与 overlay 私有 xattr。
- `inode/data.rs`：数据读写转发，读 lower 时带 `O_NOATIME`，`O_APPEND` 在每 inode 事务锁下串行。

### 14. 文件结构

```text
overlayfs/
├── mod.rs                    — 模块声明 + init
├── fs_type.rs                — OverlayFsType：VFS 注册类型
├── layer.rs                  — Layer / LayerStack / RealObjectStack：底层目录层模型
├── real.rs                   — RealObject / RealPath / RealObjectKey
│
├── fs/                       — 挂载模块
│   ├── mod.rs                — OverlayFs + FileSystem 实现
│   ├── policy.rs             — MountPolicy
│   └── mount/                — 构造流程
│       ├── mod.rs            — 构造流程编排
│       ├── options.rs        — MountOptions 解析
│       ├── layer_parts.rs    — layer root 解析与 overlap/workdir 校验
│       ├── inuse.rs          — upper/workdir 排他认领
│       └── capabilities.rs   — upper 能力测量
│
└── inode/                    — 逻辑对象
    ├── mod.rs                — OverlayInode + Inode/FileOps 实现
    ├── inode_cache.rs        — 同一底层对象到同一 OverlayInode 的复用
    ├── lookup.rs             — 按层顺序名字解析
    ├── identity.rs           — 对外 dev/ino 身份换算
    ├── readdir.rs            — merged directory 稳定枚举
    ├── copyup/
    │   ├── mod.rs            — copy-up 仲裁/制备/发布
    │   └── workdir.rs        — workdir 临时对象
    ├── dir/
    │   ├── mod.rs            — 共享命名空间变更流程
    │   ├── create.rs / link.rs / remove.rs / rename.rs
    │   └── whiteout.rs       — whiteout 表示/发布/清扫
    ├── data.rs
    ├── permission.rs
    ├── metadata.rs
    └── xattr.rs
```
