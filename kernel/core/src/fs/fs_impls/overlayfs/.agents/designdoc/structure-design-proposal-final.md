<!-- SPDX-License-Identifier: MPL-2.0 -->

# 介绍 `overlayfs` 的结构性重新实现设计

## 动机

overlayfs 是一种堆叠式联合文件系统：它把一个**可选的**可写目录（upper）与若干只读目录（lower）自上而下合并成单一可见的目录树，写入时把被改动的对象复制进 upper，从不原地修改 lower。

### Legacy 实现的主要缺憾

- **单文件 monolith**：所有功能都挤在一个 `legacy_fs.rs` 文件中，读者很难按类型定位行为，也难以扩展。
- **身份与并发不安全**：同一个底层文件被多次访问时会生成多个不同的 overlay inode，状态无法共享；并发写者可能重复执行同一操作。
- **工作目录行为不符合 Linux**：copy-up 不使用隔离的工作目录做原子替换，崩溃时可能留下半成品；多个 overlay 可能共用同一个工作目录，互相干扰。
- **功能与正确性不足**：只读 overlay、跨目录重命名、同步、超级块信息和权限检查等能力缺失；目录枚举在大目录下低效且续读位置不稳定，对外文件标识（dev/ino）也不稳定。

### 重新实现的目标

legacy 的缺憾大多源于模型组织的缺失。本设计从 overlayfs 的领域概念出发重新实现，让代码以核心类型为锚、按领域概念组织，并修复上述身份复用、并发、工作目录、枚举稳定性等问题；每一机制的持有与引用模型也随之明确。

## 设计
### 1. 总体结构

代码顶层由以下入口组成：

- `mod.rs`：模块声明与 `init`。
- `fs_type.rs`：`OverlayFsType`，VFS 注册类型。
- `fs/`：挂载模块，表示一次 mount 拥有的状态与构造流程。
- `inode/`：逻辑对象模块，表示 overlay 暴露给 VFS 的 `OverlayInode` 及其行为族。
- `real.rs`：真实对象引用模型。
- `layer.rs`：层模型；每层对底层挂载持有一份只属于自己的私有挂载视图，层根由视图承载。

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
    /// overlay 的 `AnonDeviceId` RAII guard。
    _anon_device_id: AnonDeviceId,
    /// 用于构造 root inode 的弱自引用。
    self_weak: Weak<OverlayFs>,
}
```

- `layer_stack`：overlayfs 在挂载时把多个底层目录按顺序组装成层栈，并维持一个 upper-first 的合并视图；详见 §4 层模型。
- `policy`：保存运行期挂载策略，例如只读模式和权限设置，并决定是否允许写入和 copy-up。
- `identity`：持有 dev/ino 身份换算策略，用于计算每个 overlay 对象的对外可见身份；详见 §10。
- `upper_workdir_pair`：持有可写挂载的 upper/workdir 排他认领与 workdir 暂存资源。workdir 是 upper 所在文件系统上的一个临时目录，copy-up 与 whiteout 的对象先在其中暂存，再原子改名发布到最终位置；workdir 的排他认领与准备见下方构造流程第 3 步，copy-up 的发布见 §9，whiteout 的发布与清理见 §12。
- `whiteout_cache`：与隐藏 lower 名字的 whiteout 机制相关的挂载级共享缓存；这里先只说明它服务于目录命名空间变更，详见 §12。
- `inodes`：inode 身份复用缓存；详见 §8。

### 3. 挂载构造流程 `fs/mount/`

`fs/mount/` 为一次性构造流程：

1. `options.rs`：解析 `MountOptions`。
2. `layer_parts.rs`：解析 upper/lower/workdir root path，校验并组装 `LayerStack`。校验内容包括：layer root 不能相同、不能互为祖先/后代、不能跨越挂载边界；workdir 不能与 lower roots 冲突。装配期同时为每一层以及 workdir 构造以其解析路径为根的私有挂载视图，复用 VFS 既有的挂载克隆原语。
3. `inuse.rs`：排他认领 upper/workdir 并准备 workdir，避免多个 overlay 共享同一个 upper/workdir 时互相覆盖 whiteout、临时对象和目录状态而损坏彼此的数据。
4. `capabilities.rs`：测量 upper 能力，例如 `d_type` 支持与 overlay 私有 xattr 支持，并据此决定 whiteout 表示与其他挂载策略。
5. `mod.rs`：编排以上步骤。

### 4. 层模型 `layer.rs`

overlayfs 把若干底层目录叠成一个可见命名空间：至多一个可写目录作为 **upper**，其余只读目录作为 **lower**，各层按挂载时给定的固定顺序排列，upper 在最前。`Layer` 表示其中一层：

```rust
pub struct Layer {
    /// 强持有本层的私有挂载视图；视图的根 dentry 即层根。
    mount: Arc<Mount>,
    /// 每次挂载根据 layer stack 顺序动态分配的 fs id。
    fsid: u64,
    /// 底层 fs 的设备 id。
    container_dev_id: DeviceId,
}
```

`Layer` 是对本层挂载视图的唯一强持有者，保证 overlay 存活期间底层文件系统与层根不被提前回收。层根不必是底层挂载的根，因此每层在装配期获得一个以解析出的层路径为根的私有克隆视图；私有视图不对任何挂载命名空间注册。

overlay 假设挂载期间 upper/lower 不会被绕过 overlay 直接修改（直接修改底层属于未定义行为；参见 `Documentation/filesystems/overlayfs.rst`：“directly modifying underlying filesystems could result in undefined behavior”，且 lower 期望保持只读）。

`LayerStack` 表示 upper/lower 层的有序集合，其中 `lowers` 在挂载时至少有一个：

```rust
pub struct LayerStack {
    upper: Option<Layer>,
    lowers: Vec<Layer>,
}
```

### 5. 逻辑对象 `OverlayInode`

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
    /// 记录下来的父：逻辑 overlay 父目录；同时也是 copy-up 的发布父目录。
    recorded_parent: RwMutex<Weak<OverlayInode>>,
    /// copy-up 的仲裁与发布名；详见 §9。
    copyup: Mutex<Option<String>>,
    /// VFS 提供的每 inode 扩展状态。
    extension: Extension,
}
```

字段说明：

- `lowers` / `upper`：lower 是只读层真实对象，upper 是可写层真实对象。同名目录在多层都存在时，overlay 会形成 merged directory（见 §11）。`upper: Once<RealObject>` 表示 copy-up 是 lower→upper 的单向、至多一次发布；读路径无锁，发布时原子写入。
- `object_id`：预计算的对外 dev/ino，由 `IdentityPolicy` 计算，见 §10。
- `lock`：目录事务锁与 readdir 索引域；`ReaddirIndex` 是 merged directory 的稳定枚举索引，见 §11。非目录时不携带任何附加状态，只是用来串行化对这个对象的并发访问。
- `recorded_parent`：逻辑 overlay 父目录与 copy-up 发布父目录。"记录下来的父"——它是首次绑定时写下的记录，而非每次访问时推导的事实。
  - **绑定规则**：发布坐标 `(recorded_parent, name)` 由首次正向解析在构造期一次性确立；后续 lookup 命中缓存一律不回写；仅在跨目录 rename 成功时更新一次。
  - **多 link 取舍**：同一底层对象的多个 alias 以 first-seen-wins 为准，各自收敛到各自的 upper 发布；alias 重链接（index 族）明确出界。
  - **发布坐标记录的是首次绑定时的名字**，而非某次触发 copy-up 时经过的名字；二者可以不同。不同时，物理副本落在坐标位，正主对象随之转为 upper-backed 并继续服务其现有句柄；其他别名在其后各自的解析中会被判为 stale-upper 并重建为独立的 lower-backed 实例——它们看到的来源数据停留在 copy-up 之前的时刻，直到各自再次经历 copy-up 才追上。
  - **不由底层 dentry 派生**：overlay 命名空间的演化可以领先物理形态（redirect 式“先改逻辑名、后决定是否迁移”，以及 whiteout 遮蔽、身份缓存换键等过渡时刻），该字段承载的是“overlay 认为它在哪”，不是“物理此刻在哪”。
- `copyup`：copy-up 的仲裁与发布名；详细语义见 §9。

#### 关于 `recorded_parent` 的取舍

**用处**：copy-up 依赖它沿父链逐级copyup，直到父目录存在于 upper 中；制备好的副本要有确定的落点。readdir 依赖它，使 `".."` 必须给出父目录的对外身份，而父目录是另一个 overlay 对象，当前接口并不传入。

**现写法的问题**：首绑坐标 ≠ 触发语境——写入经由 `/b/y` 而坐标记录为 `/a/x` 时，物理副本落在 `/a/x`，触发路径随后按别名分裂规则重建自己的实例。

**VFS 侧替代方案**：仿 Linux 由 dcache 承载父子关系的做法，把 `&Dentry`（或其 `NameAndParent`）作为调用期上下文经 Inode trait 传入，使每次操作的发布坐标即"本次经过的父"，持久字段删除。

### 6. 真实对象引用 `real.rs`

`RealObject` 是对某一层中真实对象的引用：`layer_index` 标识所在层，dentry 锚定
该层的具体条目；所在层的身份（fsid / container device id）由层定义统一携带，
不逐对象复制。

```rust
pub struct RealObject {
    /// 所在层序号。
    layer_index: usize,
    /// dentry 锚定的真实条目。
    dentry: Arc<Dentry>,
}
```

经由所在层的挂载视图可按需重建完整 `Path`；锚点的有效性跟随 overlay 的生命周期——只要逻辑对象可达，它引用的视图与 dentry 就不会被回收。

`RealObjectKey` 是 overlay 内部标识真实对象身份的值类型，由 `fsid` 与真实 inode 号组成，用作 inode cache 的键：

```rust
pub struct RealObjectKey {
    fsid: u64,
    real_ino: u64,
}
```

### 7. 名字解析 `inode/lookup.rs`

名字解析按层的固定顺序扫描：同名对象合并为同一逻辑对象；各层同名目录对象合并为同一对象，非目录则以最上层者为准返回；层的 whiteout 与 opaque 标记会在各自层级截断其下的贡献。

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

三个 negative 变体对外统一呈现 `ENOENT`，区别只在 fs 内部决策面：

- **Absent**：该名字在任何层都不存在；对上层就是普通的未命中，创建走 upper 的常规新建路径。
- **HiddenByWhiteout**：whiteout 是**名字级遮蔽物**——某一层里一个独立的隐藏对象（如 char device `0:0` 或带标记的零长文件），用来挡住更下层的同名者。例：upper 有名为 `foo` 的 whiteout ⇒ 该名完全不可见，枚举跳过。对上层：创建必须经 over-whiteout 制备替换，不能裸建；对它发起的删除请求只会得到 ENOENT——遮蔽物保持原位、不被触碰；rename 把这类目标识别为 whiteout 目标，反转 Replace/Exchange 选择。
- **HiddenByOpaque**：opaque 是打在某层真实目录上的**目录级标记**——它修饰的是整个真实目录而不是某个名字对象；当上层目录带此标记时，lower 的一切同名贡献都被切断。例：upper 目录带 opaque ⇒ 可见集合即其自身条目。对上层：与 `Absent` 合流 plain-create；枚举时下层目录整体退出合并。

这些标记通常由 upper 写入，但任何一层出现都会影响更下层。详细表示与发布见 §12。

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

当用户对仍由 lower 提供内容的对象发起写类操作时，overlay 先制作一份可写的 upper 副本，再把它原子地安置到该名字的位置上——这一过程称为 copy-up。制备期间的一切产物都私有地暂存在 workdir 中：发布前的崩溃不会向 upper 暴露任何半成品。

copy-up 的仲裁与发布名由一个互斥锁承载：仍为 lower-backed 的对象持 `Some(name)`，表示待发布到 `(recorded_parent, name)`；发布成功后置 `None` 并永久退役——此后该对象再无任何 copy-up 事务。"是否已发布"的唯一事实源是 `upper` 是否已设置，两者以"发布完成时一起收敛"保持一致。

copy-up 的动作面落在 `impl OverlayInode` 的四个方法上：

```rust
impl OverlayInode {
    /// 获取每对象 copy-up 互斥锁；guard 暴露待发布的名字。
    fn lock_copyup(&self) -> MutexGuard<'_, Option<String>>;

    /// 在 workdir 中暂存数据/metadata/xattr。
    fn stage_in_workdir(&self, name: &str) -> Result<WorkdirTemp>;

    /// 把暂存于 workdir 的对象原子重命名进 upper 父目录的 `name` 位置。
    fn publish_by_rename(&self, name: &str, staged: WorkdirTemp) -> Result<()>;

    fn copy_up(&self) -> Result<()> {
        if self.upper.get().is_some() {
            return Ok(());
        }
        let mut published = self.lock_copyup();
        let Some(name) = &*published else {
            return Ok(());            // 已无待发布坐标
        };
        let staged = self.stage_in_workdir(name)?;
        self.publish_by_rename(name, staged)?;
        *published = None;            // 坐标退役
        Ok(())
    }
}
```

锁序上只引入两条固定次序：先取得对象自身的 copy-up 锁，再进行祖先提升与制备；`publish_by_rename` 在持有 copy-up 锁的同时短暂获取父目录的事务锁完成物理改名与语义提交，随后释放。除这条唯一的「copy-up → 目录事务」加锁边之外，不再引入任何新的次序。

copy-up 会沿祖先目录链逐级进行：发布 child 之前，其 parent 目录已先完成 copy-up、存在于 upper 中。workdir 暂存避免暴露半成品：发布前崩溃只会留下一个私有 workdir 对象，而不会留下可见的 upper 条目。

物理改名是 copy-up 的分界线。改名之前的任何错误处理方式都相同：删掉 workdir 里的临时副本，整个过程从头再来，外界看不到任何痕迹。

改名成功之后只剩收尾一件事：在 inode 缓存里登记这个新的身份，并把逻辑对象标记为 upper-backed。并发场景遵守一条简单规则——两个任务同时登记同一份文件时，后来者等前者登完，然后直接复用同一个逻辑对象。

### 10. 用户可见身份 `inode/identity.rs`

overlayfs 需要单独的 identity 模块，因为合并多个底层文件系统后不能简单暴露底层 `st_dev` / `st_ino`：

- 不同底层文件系统可能有冲突的 `st_dev` / `st_ino`，直接透传会让用户态把不同对象误判为同一对象；
- overlay 需要根据底层能力和挂载配置选择 passthrough（直接透传底层 dev/ino）、xino 编码（对底层 ino 重新编码以避免冲突）或 fallback（回退到 overlay 分配的身份），以提供稳定且不冲突的对外身份；
- 身份在 copy-up 前后必须保持稳定：copy-up 改变物理来源，但用户看到的还是同一个 overlay 对象。

这一模块有两个核心实体：

- `ObjectId`：一个 overlay 对象的对外 dev/ino；
- `LowerIdOrigin`：copy-up 前 lower source 的持久化身份记录。

投影策略由挂载级的 `IdentityPolicy` 承载，并在装配期一次性固化。

### 11. 目录枚举 `inode/readdir.rs`

当同名目录在 upper 与 lower 中都存在时，overlay 呈现一个 **merged directory**：可见名字是 upper 与各 lower 的并集，upper 优先，元数据以 upper 为准。

readdir 的目标是提供稳定、可续的枚举顺序，避免每次 getdents（读取目录项的系统调用）都全量重扫。

合并目录的枚举分为以下几个阶段：

1. **按需构建索引**：目录第一次被枚举时，按 upper-first、lower top-to-bottom 的顺序扫描各真实层；每个名字只保留第一次出现；whiteout、opaque 目录和同名非目录会抑制或终止下层名字继续合并。
2. **稳定 cookie 序列**：`.` 与 `..` 使用固定 cookie，其余每个可见名字按序获得一个 cookie；所有 cookie 单调递增且永不复用；调用方以 cookie 作为 offset（续读位置）从上次位置续读。

重建遵循一条判据：只有无法证明既有 cookie 序仍然成立时才做全量重扫。单个名字的删除落为 tombstone，续读位置得以保留；新名的追加若能证明位于既有序列末尾则原地插入。改名是旧名 tombstone 与新名插入的组合：插入点能证明落在序列末尾时原地并入，否则才降级为全量重扫；已被暴露过的 offset 始终由 tombstone 保证不被重编号。真正的重建集中在大片可见性变化——最典型的是 opaque 标记的出现或消失使下层贡献整体增减。

### 12. 命名空间变更 `inode/dir/`

`inode/dir/` 包含所有对父目录命名空间做变更的操作：

- `create.rs` / `link.rs` / `remove.rs` / `rename.rs`
- `whiteout.rs`：whiteout 的表示、发布与清扫

删除 lower 可见名字时不能修改 lower，而是在 upper 发布 whiteout 遮蔽物。whiteout 的物理表示根据 upper 能力选择：可以是 char device `0:0`，也可以是带 `trusted.overlay.whiteout` xattr 的零大小文件。

`WhiteoutCache` 是一个挂载级、单槽的复用池，缓存的是位于 workdir 中的私有 whiteout 临时对象，用于后续发布为 upper 中的 whiteout。它的原理是：同一个 workdir whiteout 可以通过 hard link 发布到多个 upper 目录/名字下，因此 remove/rename 不必每次重新创建 whiteout 临时对象。

使用方式：

- 删除某对象时，无论 unlink/rmdir 还是 rename，只要需要在 upper 留下一个遮挡 lower 同名对象的 whiteout，都走同一条发布路径：取 workdir 里的私有临时物，原子地安置到目标名字。
- 只有当 upper 中该名字尚不存在、且底层支持 hard link 共享时，发布成功后才会把 workdir whiteout 存回缓存。
- 如果目标已有 upper 对象、需要 rename-over 发布，或者 hard link 共享失败，则 whiteout 临时对象被消费掉，不回填缓存。hard link 共享可能因 `EMLINK`（达到 link count 上限）或 `EOPNOTSUPP`（后端不支持该操作）失败。
- `can_share_by_link` 一旦因 hard link 失败降为 false 就保持禁用；后续 whiteout 都通过消费新临时对象发布。

缓存的临时对象始终位于 workdir，属于私有对象；残留的 workdir 条目会在下次挂载准备 workdir 时清理。

### 13. 权限、属性、xattr、数据

- `inode/permission.rs`：两段式权限检查——先做 overlay 本地权限检查，需要时执行 copy-up，再检查底层真实对象权限；copy-up 是两段之间的动作，不是第三段权限检查。
- `inode/metadata.rs`：属性写。
- `inode/xattr.rs`：xattr 操作与 overlay 私有 xattr。
- `inode/data.rs`：数据读写转发，读 lower 时带 `O_NOATIME`，`O_APPEND` 在每 inode 事务锁下串行。

### 14. 文件结构

```text
overlayfs/
├── mod.rs                    — 模块声明 + init
├── fs_type.rs                — OverlayFsType：VFS 注册类型
├── layer.rs                  — Layer / LayerStack：底层目录层模型
├── real.rs                   — RealObject / RealObjectKey
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
