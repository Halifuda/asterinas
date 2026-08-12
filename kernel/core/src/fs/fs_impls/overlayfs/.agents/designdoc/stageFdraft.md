<!-- SPDX-License-Identifier: MPL-2.0 -->

# Overlay FS 基础设计：Stage F 确定稿

**状态：** 已完成；BindingCache/ReaddirIndex source-of-truth、VFS 交接和
普通底层 lookup 边界均已确认
**前置：** Stage A、Stage B/C、Stage D 和 Stage E 已完成
**范围：** P0/P1 全部 55 个 Micro ID，以及随核心实现的 `P2-01 xino`
**定位：** 明确跨模块责任和 VFS 交接能力，不冻结 Rust 类型、函数签名或
Creator pass

## 1. Stage F 到底要解决什么

Stage E 已经说明“一次请求怎样走完”。Stage F 只需要再回答两个问题：

1. **每一步由哪个 Meso-component 负责？**
2. **Overlay 和现有 VFS 之间需要怎样交接，才能让这条流程真的闭合？**

可以把 Meso-component 理解成不同的责任部门，把 VFS interface 理解成部门之间
的交接单：

- 一个状态只能有一个 source of truth；
- 一个部门可以使用另一个部门的结果，但不能偷偷接管对方的状态；
- 交接单要说明什么时候交接、交接后谁负责、失败时如何退回或重新观察；
- 交接单描述语义能力，不在本阶段写成最终 Rust API。

Stage F 不重新设计 B/C 的锁序和 publication law，也不重新选择 P2/P3 范围。

## 2. 当前 VFS 的事实

这版初稿根据当前 Asterinas VFS 的实际形状整理：

- `FileSystem::root_inode()` 只返回一个 `Arc<dyn Inode>`，没有 `Result`，因此
  mount 阶段必须在调用它之前完成所有可能失败的准备；
- `Inode::lookup(name)` 只返回 `Arc<dyn Inode>`，不能同时返回 Overlay 的
  `(parent, name)` binding state；
- `Dentry` 保存一个 inode、name、parent 和目录 children cache，但目前没有
  generic filesystem-private payload；
- `CachedDentry::Negative` 只表示“没有 inode”，没有 whiteout/opaque 或其他
  filesystem-private state；
- `Dentry::lookup_child()` 当前在 children cache guard 仍然有效时调用
  `inode.lookup()`，之后再把结果写回 cache。普通底层 lookup 可以在这个可睡眠
  guard 下阻塞；只有底层实现反向进入当前 Dentry 或 Overlay lock domain 时，
  才形成需要特别处理的重入边界；
- `Inode` 已经提供 `REVALIDATE_ABSENT`、`revalidate_absent()`、metadata、
  permission 和 xattr 操作，但这些能力还不能单独表达 Overlay 的完整
  positive/negative projection；
- 现有 VFS 的 dentry children cache 是 VFS owner，不能被 Overlay private
  `BindingCache` 直接替代。

本轮确认的 source-of-truth 方向是：

- `BindingCache` 是 `(Overlay parent, name)` 的 lookup、hidden reason、authority
  和 binding 语义第一信源；
- `ReaddirIndex` 是一个 Overlay directory 的可见名称序列、cookie 和
  `NeedsRebuild` 状态第一信源；
- VFS positive/negative dentry cache 只是派生的路径加速缓存，不承担 Overlay
  binding 的最终语义，也不需要保存完整的 whiteout/opaque reason；
- basic Overlay 不以 generic VFS private payload 作为正确性的前置条件。

对应源码入口是：

- `kernel/core/src/fs/vfs/fs_apis/file_system.rs`：root publication boundary；
- `kernel/core/src/fs/vfs/fs_apis/inode.rs`：inode operations、revalidation、xattr
  和 permission；
- `kernel/core/src/fs/vfs/path/dentry.rs`：dentry tree、children cache 和 lookup
  publication；
- `kernel/core/src/fs/vfs/fs_apis/xattr.rs`：xattr namespace 和 set flags。

## 3. 跨 Meso 的责任分工

下面的表是 Stage F 的主要责任结果。它规定“谁负责什么”，不是要求每一行都
变成一个 Rust struct。

| 请求或状态 | 主责任 Meso | 它拥有的语义 | 它不接管的内容 |
| --- | --- | --- | --- |
| mount option、effective policy | `mount_options` | 解析输入、形成 immutable policy | 不执行 copy-up 或 lookup |
| layer、upper/workdir、mount lifetime | `mount_layers_lifecycle` | layer snapshot、claim、root inputs、teardown | 不拥有 named binding 或 file data |
| upper exclusivity、fsid、durability | `upper_exclusivity_durability` | 共享 upper/workdir claim、mount identity 和 sync policy | 不拥有 namespace visibility |
| root/name projection、identity | `identity_and_carriers` | root carrier、Overlay inode、object identity、xino projection | 不反向从 inode 找 name |
| upper-first lookup、negative/hidden state | `path_lookup_visibility` | visibility reduction、BindingCache、revalidation evidence | 不写 whiteout、不执行 copy-up |
| visible directory sequence | `merged_readdir_cache` | ReaddirIndex、cookie、dedup、NeedsRebuild | 不拥有 physical whiteout |
| authority transition | `copy_up_lifecycle` | copy-up winner/waiter、workdir temporary、upper publication | 不拥有最终 file I/O 或 directory index |
| file view、page cache forwarding | `file_io_page_cache` | read/write/mmap/fallocate 等 delegation 和 real handle | 不创建第二个 page cache |
| metadata、permission | `inode_attributes_security` | local check、creator-credential handoff、metadata mutation | 不重新定义 copy-up lifetime |
| xattr classification | `xattr_namespace_escaping` | public/private 分类、namespace/filter/escape policy | 不拥有 origin/index 或 whiteout truth |
| namespace mutation | `directory_mutations_whiteouts` | create、unlink、rmdir、link、rename、whiteout/opaque publication | 不重新实现 identity 或 copy-up owner |
| origin/index/export seam | `origin_index_export` | 后续 lower-upper identity 和 export 依赖边界 | 不改变 basic visibility owner |
| metacopy/data-only seam | `metacopy_verity_data_layers` | 后续 metadata/data authority 分离 | 不创建第二套 data/page-cache owner |

跨模块时只有一个总原则：**调用方可以拿到结果，但结果的长期状态仍归原 owner
所有。** 例如 mutation 可以请求 copy-up，但不能把 copy-up state 复制到自己的
临时 carrier 中。

## 4. 一条 lookup 的 VFS 交接

### 4.1 为什么 Overlay 需要本地 BindingCache

对普通 filesystem，`Inode::lookup(name)` 返回一个 inode 已经够用。Overlay 的
lookup 还需要同时记住：

- 这个 name 属于哪个 Overlay parent；
- upper 和 lower 的观察结果；
- 这是 single object 还是 merged directory；
- name 是 absent、whiteout-hidden 还是 opaque-hidden；
- 后续 revalidation 应检查哪些 evidence；
- positive result 应该绑定哪个 Overlay inode identity。

这些信息由 Overlay 本地 `BindingCache` 保存。`Inode::lookup(name)` 只需要从
BindingCache 得到当前 projection，并返回对应的 Overlay inode 或 `ENOENT`；
VFS dentry cache 随后缓存这个结果，但不成为 binding 的第一信源。这样可以让
Overlay 继续使用当前 VFS 的 inode-only lookup 形状，同时把 whiteout/opaque、
authority 和来源 layer 留在 Overlay 自己的 owner 中。

这个方向要求 mutation 在释放 `DIR` 前同步更新或失效 BindingCache、barrier、
identity 和 ReaddirIndex，并把 VFS dentry cache 当作需要同步修正的派生视图。

### 4.2 推荐的语义交接：本地 cache first

Stage F 推荐把一次 lookup 看成一次“Overlay 本地 cache first”的流程：

```text
VFS 需要 (parent, name)
    -> Overlay 取得自己的 parent DIR
    -> 先查 BindingCache
    -> cache hit：直接得到 positive/negative projection
    -> cache miss 或 stale：观察 upper/lower 并重建 BindingCache
    -> positive result 做 identity projection
    -> 在释放 DIR 前更新 BindingCache、barrier、identity 和 ReaddirIndex
    -> 返回 inode 或 ENOENT，供 VFS 形成派生 dentry cache entry
```

这里的本地 cache first 是 Stage F 的语义决定，不是冻结的 Rust 函数签名。它
解决两个问题：

1. lookup 的 semantic answer 不依赖 VFS dentry 是否携带 Overlay private payload；
2. VFS 看到的 positive/negative result 总是由已经发布的 BindingCache projection
   产生，而不是由 VFS cache 自己推导 whiteout/opaque 或 authority。

cache miss 需要 underlying BIO 时，可以在现有 generic children-cache guard 和
Overlay parent `DIR` 的保护范围内直接访问底层 inode。这个调用可以阻塞，但普通
底层 filesystem lookup 不会因此产生重入，也不需要为了 BIO 释放并重新取得当前
guard。实现必须保持底层 lookup 的调用方向为单向：它不能通过 callback 或路径
解析重新进入当前 Dentry、BindingCache 或同一个 Overlay `DIR`。只有在具体实现
已经证明存在这种同步重入或反向锁序时，才需要使用 pinned references、释放/重试
和完整 revalidation 的 adapter。这个局部规则不改变 BindingCache 是第一信源的
决定，也不要求把 generic private payload 提升为 basic 语义 owner。

### 4.3 Positive publication

positive lookup 的语义关系由 BindingCache first 建立，VFS 只得到它的派生视图：

```text
(parent, name)
    -> BindingCache positive binding
    -> Overlay inode identity
    -> current real authority/provenance
    -> VFS positive dentry cache view
```

多个 hard-link name 可以共享一个 Overlay inode，但每个 `(parent, name)` 仍有
自己的 binding state。copy-up 只更新 authority/provenance，不重新创建已经发布
的 logical inode、name 或 cookie。

### 4.4 Negative publication

negative lookup 不创建 inode，但 BindingCache 不能丢掉隐藏原因。至少需要区分：

```text
Absent
HiddenByWhiteout
HiddenByOpaque
```

对 VFS 来说它们都表现为 negative dentry；对 Overlay 来说，BindingCache 中的
原因会影响之后的 revalidation、create-over-whiteout、unlink、rename 和
readdir barrier。VFS negative dentry 不需要携带完整的隐藏原因，因为它不是
Overlay 的语义 owner。

## 5. Revalidation 交接

当前 VFS 已有 `REVALIDATE_ABSENT` 和 `revalidate_absent(name)`，basic Overlay
采用保守 baseline：

- 对 negative cache hit 暂时直接返回 `false`，让 VFS 丢弃 negative entry 并
  重新进入 Overlay lookup；
- 重新 lookup 首先查询 BindingCache，仍然 absent/hidden 时不必重新扫描所有
  layer，cache miss 或 stale 才进行 underlying observation；
- `revalidate_absent` callback 本身必须保持 cheap，不能在 generic children-cache
  guard 下执行 underlying I/O；需要 BIO 的检查返回 `false`，交给完整 Overlay
  lookup；普通完整 lookup 可以按上一节规则直接访问底层 inode；
- positive dentry 的 BindingCache 失效仍由 Overlay mutation publication 负责，
  不能只依赖 VFS 的 negative revalidation。

这里不需要引入全局 version。parent `DIR` 串行化、定点失效和 lookup fallback
已经构成正确性 baseline。

## 6. 其他 VFS 交接

### 6.1 Root

mount Builder 在 `FileSystem::root_inode()` 前完成 layer、upper/workdir、identity、
xino 和 root projection 的所有 fallible 准备。`root_inode()` 只发布已准备的
root carrier；失败释放由 mount construction 负责，不让 root callback 承担
rollback。

### 6.2 Directory mutation

当前 VFS 的 `Dentry::create`、`link`、`unlink`、`rename` 等操作会管理 children
cache 并调用 inode operation。Stage F 要求 Overlay filesystem operation 在进入
physical upper operation 前建立自己的 parent `DIR` consistency domain，因为当前
VFS 不会自动提供 Linux 风格的 parent-directory lock。

成功后，Overlay BindingCache、barrier、identity 和 ReaddirIndex 是第一信源，
VFS children cache 是必须同步修正的派生视图；这些更新都必须在 parent `DIR`
释放前完成。若 VFS 当前接口不能与 Overlay state 做到同一时序，Stage F 需要
记录明确的 cache invalidation/update seam，而不是让每个 mutation 自己猜测
如何修 cache。

### 6.3 File view 和 page cache

VFS file operation 只拿到当前 authority 对应的 real view：

- read-only operation 可以使用 lower view；
- write-capable operation 必须先完成 copy-up trigger；
- writable page-cache/mmap inlet 只能在 upper authority 确认后开放；
- Overlay 不向 VFS 提供第二个 page-cache backend；
- release 只释放 real handle/view，不承担 Overlay authority lock。

### 6.4 Metadata、permission 和 xattr

现有 `Inode` 的 metadata、`check_permission` 和 xattr methods 可以作为
underlying delegation 入口，但 Overlay 需要在调用它们之前补上自己的语义层：

```text
Overlay-local check
    -> current real authority / copy-up
    -> creator-credential underlying check
    -> metadata or xattr operation
```

private xattr 的分类、过滤和 Overlay owner authorization 属于
`xattr_namespace_escaping`，不能交给通用 VFS `list_xattr` 自动决定。

## 7. Stage-E 工作流到 VFS/Meso 的交接表

| Stage-E 工作流 | VFS 入口能力 | Overlay 主责任 | 关键交接 |
| --- | --- | --- | --- |
| mount/root | eager `root_inode` | mount/layer + identity | Builder 完成后发布 root carrier |
| lookup | inode/ENOENT lookup、positive/negative cache view | lookup + identity | BindingCache first，VFS dentry 只缓存派生结果 |
| stat | inode metadata view | identity + attributes | xino projection 不反查 name |
| readdir | offset、dirent visitor | merged readdir | visible-only index 和 xino `d_ino` |
| open/read/write | real file view、page cache | copy-up + file I/O | trigger 后重新取得 current authority |
| permission/metadata | permission、metadata callbacks | attributes/security | local check 先于 side effect |
| xattr | get/set/list/remove | xattr policy | private/public 分类不泄漏 |
| create/unlink/rmdir | dentry children update | mutation + readdir | binding/barrier/index 同步发布 |
| link | multiple dentries → one inode | mutation + copy-up | upper hardlink 正常；no-index lower alias 降级 |
| rename | one/two parent dentry updates | mutation | 默认 lower/merged directory cross-dir `EXDEV` |
| fsync/teardown | sync and lifetime release | durability + mount lifetime | runtime release 与 durable cleanup 分离 |

## 8. Stage F 不做什么

- 不新增 production Rust 类型；
- 不把当前推荐的 reservation/publication 语义直接改写成最终函数签名；
- 不重新设计 `DIR -> CUL -> INODE -> WL -> UPPER`；
- 不把 Overlay private BindingCache 变成第二套 VFS children cache；
- 不实现 redirect_dir、metacopy、index 或 NFS export；
- 不创建或修改 ktest、filesystem-local test 或运行时验证；
- 不进行 Creator pass slicing。

## 9. 已确定的 Stage-F 决策

Stage F 的接口闭包和调用边界确定如下：

1. `BindingCache` 是 `(parent, name)` 的 lookup、hidden reason、authority 和
   binding 第一信源；`ReaddirIndex` 是目录可见序列和 cookie 第一信源；VFS
   positive/negative dentry cache 只是派生路径缓存；
2. BindingCache hit 不访问底层 FS；cache miss 可以在已有 dentry children
   guard 和 Overlay parent `DIR` 下直接执行底层 lookup。普通底层 lookup 不需要
   release/retry；只对已证明的同步重入或反向锁序设置 adapter；
3. `revalidate_absent` 固定采用“直接返回 `false`，重新 lookup；lookup 优先查
   BindingCache”的保守策略；
4. xino overflow 不在基础设计中展开，具体算法留给实现阶段参考 Linux；完整
   UUID modes 仍作为独立范围问题处理。

## 10. Stage-F 完成结论

Stage F 的核心产物不是一套新代码，而是两张表、一个 source-of-truth 决定和
一个确定的 VFS 调用边界：

1. Stage-E workflow → Meso owner → VFS handoff table；
2. owner、publication、failure/retry 和 lifetime 的 cross-meso boundary table；
3. BindingCache/ReaddirIndex 为第一信源、VFS dentry 为派生视图，以及
   `revalidate_absent` 保守重查的语义能力清单。
4. 普通 cache miss 可阻塞访问底层 inode；只有已证明的同步重入或反向锁序才需要
   release/retry adapter。

Stage F 已完成，不再拆分额外的 traceability 或 design-review 阶段。相关的
traceability、xfstests evidence 和综合一致性检查继续使用已经接受的
Architect/Designer artifacts 及本稿中的跨模块表，不新增设计阶段。实现前仍需按仓库协议单独完成必要的 Designer wording
repair、main-agent pass slicing 和后续 Creator/Checker 流程；这些是执行准备，
不是新的设计阶段。
