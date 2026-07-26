<!-- SPDX-License-Identifier: MPL-2.0 -->

# B/C-1：Mount、Layer Stack、Upper/Workdir

**对应 meso-components：**

- `mount_options`
- `mount_layers_lifecycle`
- `upper_exclusivity_durability`

**设计主题：**

- mount 配置和 layer stack 的所有权；
- upper/lower 的顺序和只读状态；
- upper/workdir 的生命周期、同层约束和挂载独占；
- mount 构造成功、失败回滚和 teardown；
- mount 级 credential、身份策略和持久化协调的归属；
- mount 阶段可以阻塞的底层调用及其发布边界。

#### 1. Mount 顺序总览

mount 阶段需要建立这些状态，但必须区分“选择 policy”和“执行可能改变 upper 的
操作”。建议顺序如下：

```text
1. 解析 options，选择 credential / read-only / fsync / UUID policy
2. 解析并验证 layer stack
3. 验证 upper/workdir 关系
4. 计算 effective read-only 和 durability policy
5. 登记 upper/workdir 独占
6. 准备 workdir
7. 确定 UUID 和 fsid
8. 准备 root projection
9. 持久化 UUID（如果需要）
10. 发布 Ready mount
11. 创建 VFS root inode 并挂入 mount tree
```

#### 2. 解析 options 和 credential

首先解析 mount options，并确定：

- lower、upper、workdir 的输入；
- mount 是否强制 `RDONLY`；
- fsync 模式；
- UUID 模式；
- credential 来源。

credential 应尽早保存，因为后续 mount 内部访问 underlying FS 时需要知道应该使用
哪一套 credential。但这里的“保存 credential”只是选择并复制 credential policy，
不代表已经访问底层 inode。

如果后续发现 options 非法，此时只需丢弃配置，不需要 rollback。

#### 3. 解析 layer stack

接着解析所有真实路径：

```text
upper（如果存在）
lower[0]
lower[1]
...
```

并固定：

- upper/lower 的顺序；
- 每层真实 root；
- 每层底层 filesystem identity；
- 每层生命周期引用；
- root projection 所需的真实对象。

此阶段只做解析和检查，不修改 upper/workdir，也不建立独占登记。

失败时释放已经解析的 Path/inode 引用，丢弃临时 layer stack，mount 失败。

lower 可以被多个 Overlay mount 共享，因此不需要独占。

#### 4. 验证 upper/workdir

如果没有 upper：

```text
Overlay 必须是只读
workdir 没有意义
不建立 upper/workdir 独占
```

如果有 upper，则验证：

- upper 是目录；
- workdir 存在且是目录；
- upper/workdir 位于同一底层 filesystem；
- 两者不相同；
- 两者不互为祖先；
- upper 具备所需 xattr 和目录项能力；
- upper filesystem 本身可写。

这一步仍然主要是非修改性检查。失败时不建立独占，释放 layer 引用，mount 失败。

#### 5. 计算只读和 fsync policy

此时确定最终 policy：

```text
effective_readonly =
    没有 upper
    或用户指定 RDONLY
    或 upper/workdir 不具备写条件
```

fsync policy 也在此时归一化：

- 没有 upper 时，fsync 写入策略没有实际意义；
- 有 upper 时，保存 `auto`、`strict` 或其他已支持模式；
- 具体 copy-up 和 fsync 行为由后续模块执行。

这一步不执行真正的 fsync，只是确定规则。

#### 6. 建立 upper/workdir 独占

只有在前面的对象和关系检查全部通过后，才登记 upper/workdir 使用权。

独占状态不能放在 Overlay 全局 registry 中。由于 Asterinas 当前没有一个对所有
底层 filesystem 都保证稳定、可释放的 inode runtime claim carrier，这里保留两个
可选方案；两者都必须向 `OverlayMount` 提供统一的 claim guard 语义，冲突时返回
`EBUSY`。

如果登记 upper 成功、登记 workdir 失败，则必须先撤销 upper 登记，再返回失败。

这一阶段之后，才允许修改 workdir 或写 upper 私有 xattr。

##### 6.1 可选方案 A：Inode Extension runtime lease

在真实 underlying inode 的 `Extension` 中增加专用的 claim state，或使用等价的
inode-owned runtime carrier。claim state 必须包含可并发保护的 owner/token，并由
`MountBuilder` 持有 RAII guard：

```text
try_claim(inode identity)
    -> success: guard owned by MountBuilder
    -> conflict: EBUSY
commit
    -> guard moves to OverlayMount
abort/drop
    -> guard releases the claim
```

这不是把 claim 放在 Dentry 上。它要求底层 filesystem 对同一个真实 inode 提供
稳定 identity；不能只假设每个 `Arc<dyn Inode>` 都天然代表全局唯一对象。当前
Asterinas 的 Ext2 有按 inode number 的强 inode cache，Ramfs 的目录项持有 inode
`Arc`，Virtiofs 则按 FUSE node ID 使用可回收 cache。若这些 backend 被纳入 upper
支持范围，应把这种 identity/lifetime 保证写成 VFS 集成契约；否则需要由
filesystem-owned identity table 补足。

当前 `Inode::Extension` 只有 FS event 和 FS lock 两个 group，因此需要新增专用
group 或等价 carrier。仅增加一个不可释放的 `Once` 不足以表达 unmount 时的 RAII
释放。

##### 6.2 可选方案 B：xattr persistent reservation

在 upper 和 workdir 的真实 inode 上使用 Overlay 私有 xattr，例如带随机 token 的
`trusted.overlay.claim`，并使用 `CREATE_ONLY` 建立 reservation。两次写入都成功后，
`MountBuilder` 才可继续准备 workdir；正常 teardown 时校验 token 后删除对应 xattr。

这个方案跨 inode wrapper 和进程可见，且不要求新增 VFS runtime carrier，但它的
语义是持久 reservation，不是完整 runtime lease：

- upper 和 workdir 是两个 inode，两个 xattr 写入没有事务；
- kernel crash 或异常 teardown 可能留下 stale reservation；
- 后续 mount 必须 fail-closed，或依赖显式 recovery 清理；
- xattr 支持、权限和 writeback/sync 语义由底层 filesystem 决定；
- `Drop` 不能保证完成可能阻塞的 xattr 删除。

因此方案 B 只有在设计明确接受“崩溃后保守阻塞和人工恢复”时才成立，不能把它
静默描述为与方案 A 等价的 RAII claim。UUID xattr 仍是独立的持久身份记录，不应
与这个临时 reservation 混用。

##### 6.3 方案选择和共同边界

这两案都是尚未冻结的可选方案，不应在本阶段假定其中任何一案已经是 Asterinas
的现成能力。方案 A 可以提供更接近 runtime lease 的语义，但需要先扩展 VFS 的
inode-owned carrier，并为 upper 支持范围补齐稳定 identity 契约；方案 B 不需要
新增 runtime carrier，却必须接受持久 reservation 的崩溃恢复和 teardown 限制。
最终选择应结合 upper 支持的 filesystem 集合、inode identity 契约和 crash-recovery
目标确认。

无论采用哪一方案，mount preparation 都通过 `Path`/`Inode` 读取 metadata、遍历
workdir、读写 xattr 或执行目录操作；不直接修改 VFS Dentry 的 parent、children
cache 或 flags。Dentry 继续只承担路径和命名空间 cache 责任。

##### 6.4 Asterinas 当前能力基线

当前 VFS 的 `Inode::extension()` 暴露 `Extension`，但 `Extension` 只有两个一次
初始化的 group，现有 `InodeExt` 实现分别用于 FS event publisher 和 FS lock
context；它们不是可回收的 per-inode claim slot。因而方案 A 的“新增专用 group 或
等价 carrier”是明确的 VFS 设计工作，不是可以直接调用的现有接口。

VFS Dentry 只保存 inode 引用、name/parent、children cache 和挂载相关 flags，既
没有跨 wrapper 的稳定 real-inode identity，也没有 upper/workdir claim 生命周期
接口。mount 阶段可以读取通过 `Path`/`Inode` 获得的对象，但不应把 Dentry cache
当作跨 mount 的独占登记表。

VFS xattr API 已提供 `CREATE_ONLY`、读取和删除操作，因此方案 B 在支持相应 xattr
namespace 的 underlying filesystem 上具备接口基础；但默认 `Inode` 实现可以返回
`EOPNOTSUPP`，namespace、权限、持久化和 writeback 语义仍由具体 filesystem 决定。
因此 xattr 方案必须先验证 backend capability，并把“不支持 xattr”作为 mount
失败或明确降级条件，不能宣称它对所有 Asterinas filesystem 通用。

#### 7. 准备 workdir

独占成功后处理 workdir：

- 检查是否为空；
- 识别并清理允许清理的 Overlay 临时残留；
- 拒绝未知内容；
- 建立后续 copy-up 所需的内部状态。

upper 中已有的用户文件不需要清理，也不需要复制 lower。

如果 workdir 准备失败，mount 不发布，释放 upper/workdir 独占，释放 layer 引用。

如果清理过程中已经修改了 workdir，失败后不承诺把所有修改恢复原状。

#### 8. 确定 UUID 和 fsid

这一阶段依赖已经确定的 layer identity 和 upper/workdir policy。

##### UUID

根据 UUID policy：

1. 从 upper root 读取已有 Overlay UUID；
2. 如果没有且 policy 允许，生成新的随机 UUID；
3. 暂存在内存中；
4. 必要时准备写入 upper root 的 Overlay 私有 xattr。

##### fsid

根据 UUID policy 和 layer identity 计算 Overlay 的 `fsid`：

```text
uuid=null/off
    => 可以使用指定底层 layer 的 fsid

uuid=on/auto 且成功建立持久 UUID
    => 使用 Overlay 自己的稳定 identity
```

`fsid` 是运行时 `SuperBlock` identity，UUID 才可能同时有 upper xattr 这一份
持久化副本。

如果 UUID 读取、生成或 fsid 计算失败，释放独占，不发布 mount，释放 layer 引用。

#### 9. 准备 root projection

这里区分两个 root：

1. **真实 layer root**：在解析 layer stack 时已经准备好；
2. **Overlay root inode**：VFS 对外看到的 root inode。

Overlay root inode 需要使用：

- layer stack；
- read-only policy；
- credential policy；
- UUID/fsid；
- root projection inputs。

所有可能失败的 root 准备工作，都应该在 mount 返回 `Ready` 之前完成。原因是
Asterinas 的 `FileSystem::root_inode()` 没有 `Result`。

因此，`root_inode()` 只能构造已经准备好的 root carrier，不能在其中首次解析 layer、
检查 workdir 或执行可能失败的 mount 逻辑。

#### 10. 持久化 UUID

如果 UUID policy 要求将 UUID 写入 upper，建议把这一步放在所有其他可失败的 root
准备工作之后：

```text
生成/读取 UUID
    ↓
准备 fsid
    ↓
准备 root projection
    ↓
写入 upper.uuid xattr
    ↓
发布 Ready
```

这样可以避免 root 准备失败后留下新 UUID。但一旦 xattr 写成功，后续 mount attach
失败，也不要求删除 UUID，因为 UUID 是合法的持久身份记录，不是普通临时修改。

#### 11. 发布 Ready mount

此时一次性发布：

```text
OverlayMount {
    layer_stack,
    upper_workdir,
    exclusive_ownership,
    root_inputs,
    credential_policy,
    uuid_policy/runtime_uuid,
    fsid,
    effective_readonly,
    fsync_policy,
}
```

`FsType::create()` 只有在这里之后才能成功返回。其他 Overlay 组件不能看到一个
部分构造的 mount。

#### 12. 创建 VFS root 并挂载

Asterinas 的 `Mount::new()` 会立即调用 `root_inode()`。此时只做：

```text
使用 RootProjectionInputs 创建 Overlay root inode
创建 root dentry
建立 VFS Mount
```

如果 `Mount::new()` 失败，Ready mount 的引用被释放，upper/workdir 独占也随之释放。

如果 mount 已经成功挂入 VFS，之后发生的错误就不再叫“mount rollback”，而是普通
运行期错误或 teardown。

#### 13. 失败回滚总原则

失败时按创建顺序反向处理：

```text
root 尚未发布：
    丢弃 root 临时状态
    释放 UUID/identity 临时状态
    释放 workdir 临时状态
    释放 upper/workdir 独占
    释放 layer stack 引用
    丢弃 credential snapshot
    mount 失败
```

这里只保证 Overlay 的运行时状态不泄漏，不保证撤销所有已经发生的底层 filesystem
修改。例如 workdir 清理或 UUID xattr 写入已经成功后，Overlay 不提供跨操作事务
回滚。

#### 14. RAII、Builder 和发布提交

mount 构造适合使用一个只存在于构造阶段的 `MountBuilder`，但不冻结具体 Rust
字段或函数签名。Builder 持有所有尚未发布的运行时资源：

```text
MountBuilder
 ├── layer references
 ├── credential snapshot
 ├── upper claim guard
 ├── workdir claim guard
 ├── workdir preparation state
 ├── candidate UUID/fsid
 └── root projection inputs
```

构造顺序为：

```text
prepare options
prepare layers
validate upper/workdir
claim upper/workdir
prepare workdir
prepare UUID/fsid
prepare root projection
persist UUID if needed
commit MountBuilder
```

在 `commit` 之前，普通错误路径不应依赖分散的手工清理；Builder 被丢弃时，
其拥有的 layer 引用、credential snapshot 和 claim guard 自动释放。成功时，
这些资源转移给 `OverlayMount`，继续承担 mount lifetime。

资源释放必须遵守创建依赖的逆序。应通过明确的字段声明顺序、嵌套 owner 或
显式 abort/commit 协议保证这一点，不能把释放顺序留给偶然的实现细节。概念上
应当是：

```text
root temporary state
    ↓
UUID/identity temporary state
    ↓
workdir temporary state
    ↓
workdir claim
    ↓
upper claim
    ↓
layer references
    ↓
credential snapshot
```

RAII 只负责运行时所有权的最终释放，不等于底层 filesystem 事务回滚：

- 方案 A 的 claim guard 可以在 `Drop` 中清除 inode-owned runtime state；
- 方案 B 的 xattr reservation 只能在正常 teardown 中显式、尽力删除，不能把
  `Drop` 当作 crash recovery 或阻塞 VFS 操作的保证；
- `Arc` 引用可以在 `Drop` 中释放 layer、inode 和 mount lifetime；
- workdir 临时对象的删除需要显式的 prepare/abort 操作，因为 `Drop` 不能返回
  `Result`，也不应隐式执行可能阻塞或重入的 underlying VFS 调用；
- UUID xattr 一旦写入成功，不由 `Drop` 自动删除；UUID 是合法的持久身份记录；
- 已经发生的 upper/workdir 持久化修改不承诺通过通用 RAII 机制恢复。

因此，`commit` 是唯一的 mount state 发布点；`commit` 之后只允许进入不会再
产生 mount 语义失败的 root carrier 创建和 VFS attach 阶段。若 attach 失败，
已发布的 `OverlayMount` 仍通过其 lifetime/claim owner 释放运行时资源。

#### 15. 与后续模块的交接

B/C-1 向 B/C-2 提供：

- 不可变的 `OverlayLayerStack`；
- root upper/lower 真实对象引用；
- mount lifetime；
- effective read-only policy；
- layer identity/fstype 信息；
- credential policy；
- upper/workdir 使用权及其生命周期；
- UUID/fsid policy；
- fsync policy。

B/C-2 不应重新解析 mount options、重新判断 upper 是否存在、建立第二份 layer
stack、自己决定 mount 是否可写，或在 mount lifetime 失效后继续使用真实 layer 引用。

B/C-4、B/C-6 后续会消费 upper/workdir 使用权、writable policy、workdir 临时对象
约束和 mount-wide durability policy。

在这一阶段构造时需要直接构造出UPPER和WL锁的保护状态供后续模块使用。

#### 16. B/C-1 当前状态

这是 mount 顺序、发布点和逆序回滚的讨论初稿，已确认。upper/workdir claim
carrier 仍未冻结；本稿已记录 Inode Extension runtime lease 和 xattr persistent
reservation 两个可选方案，以及它们不同的崩溃和 teardown 语义。
