<!-- SPDX-License-Identifier: MPL-2.0 -->

# Overlay FS 基础设计：Stage A 草稿

**状态：** Stage A 综合草稿，待设计交流会审阅
**范围：** 基础 Overlay FS 的总体目标与语义模型
**定位：** 自顶向下设计文档的第一阶段，不冻结 Rust 类型、函数签名或实现步骤

## 1. 设计目标

Overlay FS 是一种混合式堆叠文件系统。它把一个可选的可写 upper directory
tree 和一个或多个只读 lower directory tree 组合起来，向用户呈现一个统一的
可见目录树。

本阶段定义用户可观察的基础语义：

- upper/lower 的优先级和目录合并规则；
- 路径名称如何形成可见命名空间；
- Overlay 对象如何编排多个真实底层对象；
- 读操作和修改操作如何跨越 Overlay 与 underlying VFS 的边界；
- upper 的可写条件和已有内容的含义；
- Overlay 对持久化和崩溃一致性的承诺边界。

Overlay FS 不创建独立的持久化文件系统格式。Overlay 的 mount、inode、dentry
和其他 projection 是运行时状态；真实文件数据、普通元数据以及 upper 中的
Overlay 持久化标记由 underlying filesystem 保存。

## 2. 基本术语

### 2.1 Upper 和 lower directory tree

本文使用 directory tree 而不是严格意义上的 filesystem。upper 和 lower 可以
只是某个底层文件系统中的目录，不要求是底层文件系统的根目录；多个 layer
也可以位于同一个底层文件系统上。

- **Upper tree：** 最高优先级的可写来源，存在时位于 layer stack 的最上方。
- **Lower tree：** 一个或多个只读来源，按从高到低的优先级排列。
- **Layer stack：** Overlay mount 在生命周期内持有的有序层集合。查找从
  upper 开始，再依次查找优先级更高到更低的 lower。

Lower tree 本身不要求底层文件系统可写。一个 lower tree 也可以来自另一个
Overlay FS；嵌套层的扩展属性转义等细节属于后续范围。

### 2.2 可见命名空间

可见命名空间不是 Linux mount namespace，也不是进程 namespace。它表示：

> 在 Overlay mount 根目录下，每个路径名最终映射到什么可观察文件系统对象
> 或不可见结果的关系。

它包含以下信息：

- 路径是否存在；
- 路径对应的对象类型；
- 目录中哪些名称可见；
- 同名对象中哪个 layer 胜出；
- 哪些 lower 对象被 whiteout 或 opaque 规则隐藏；
- 哪些目录需要形成 merged directory。

命名空间是外部可观察的语义映射，不等同于某个具体的底层 inode，也不等同于
文件数据。

### 2.3 Overlay projection 与真实对象

Overlay projection 是 Overlay 内部承载某个可见路径结果的运行时状态。它可以
记录或引用：

- 当前可见的 upper 对象；
- 仍然可见的、按 layer 顺序排列的 lower 对象；
- 对象是否是 merged directory；
- whiteout、opaque 等可见性事实；
- 是否已经完成 copy-up；
- 当前操作应访问哪一个真实对象。

命名空间和 projection 必须保持概念上的区分：命名空间是外部结果，projection
是实现该结果的内部状态。一个 projection 不一定对应一个底层 inode；merged
directory 至少需要协调多个真实目录对象。

Overlay projection 也不是第二套独立的数据存储。真实 inode、dentry、文件数据
和持久化元数据仍由 underlying filesystem 拥有。

## 3. Layer 与可见性语义

### 3.1 非目录对象

同一路径在多个 layer 中出现时，从最高优先级开始判断：一旦得到非目录对象，
该对象成为该名称的终止结果，更低层的同名对象被隐藏。

因此，upper 中的普通文件可以隐藏 lower 中同名的目录；upper 中的目录也会
优先于 lower 中同名的非目录对象。

### 3.2 目录对象

当同一路径在 upper 和 lower 中都对应目录，且没有可见性屏障时，Overlay 形成
merged directory：

```text
upper:  /usr/bin/a
        /usr/bin/b

lower:  /usr/bin/b
        /usr/bin/c

visible: /usr/bin/a
         /usr/bin/b
         /usr/bin/c
```

merged directory 只合并目录名称列表，重复名称只保留最高优先级的一个。目录
本身的元数据和扩展属性由 upper 目录提供，lower 目录的对应属性不作为可见
属性。

### 3.3 Whiteout 与 opaque

Overlay 不能修改 lower，因此删除 lower 内容必须由 upper 表达：

- **Whiteout：** 隐藏某个同名 lower 对象，whiteout 自身不出现在可见目录中；
- **Opaque directory：** 阻止同名 lower 目录参与合并；
- **Non-directory：** 对同名 lower 内容天然形成覆盖屏障。

这些标记是 upper 中的持久化 Overlay 元数据，由 Overlay 解释，但实际存储由
underlying filesystem 负责。

## 4. Overlay 与 underlying VFS 的语义边界

Overlay 位于 VFS-facing operation 和 underlying VFS operation 之间，承担一层
明确的语义协调，而不是把操作无条件地直接转发给某一个底层对象。

| 操作类别 | Overlay 需要先决定的语义 | 底层操作方向 |
| --- | --- | --- |
| lookup/stat | 可见对象、来源 layer、对象类型 | 读取候选层的真实对象和属性 |
| read | 当前数据来源 | 从 upper 或选定 lower 读取 |
| readdir | 合并目录、去重、隐藏 whiteout | 依次读取相关真实目录 |
| write | 是否需要 copy-up、upper 目标 | 在 upper 准备后写入 upper |
| 元数据修改 | 修改对象是否已位于 upper | copy-up 后修改 upper |
| create | 父目录的 upper 位置和发布方式 | 在 upper 创建真实对象 |
| unlink/rmdir | lower 删除如何表示 | upper 删除、whiteout 或 opaque |
| rename | 源和目标的可见性及是否允许 | 对 upper 执行相应真实操作 |

因此，Overlay 自己实现面向 VFS 的语义 operation；operation 在完成可见性、
目录合并、copy-up 和 upper 状态编排后，再调用适当的 underlying VFS operation。

Overlay 不拥有另一份独立的文件数据。其本地状态的职责是解释和协调多个真实
对象，并在语义状态稳定后发布 projection 更新。

## 5. upper 的可选性和已有内容

### 5.1 没有 upper

没有显式配置 upper 时，Overlay 是只读的：

- lower 内容仍可被读取和遍历；
- lower 所在的真实文件系统即使本身可写，Overlay 也不能通过它修改数据；
- 所有会修改命名空间、文件数据或对象元数据的操作，都必须在改变任何 lower
  对象之前被拒绝。

### 5.2 配置 upper

显式配置 upper 时，必须同时满足合法的 upper/workdir 挂载条件。workdir 应是
upper 所在底层 mount 上的独立目录，并满足 Overlay copy-up 所需的能力要求。

upper 不是空白 staging 区。挂载时 upper 中已经存在的内容就是该 mount 的
upper 内容：

- upper 普通文件覆盖 lower 同名对象；
- upper 目录与 lower 同名目录合并，除非 upper 目录为 opaque；
- upper whiteout 隐藏 lower 同名对象；
- upper 目录的元数据和扩展属性对外可见。

挂载不会把整个 lower tree 预先复制到 upper。copy-up 按需发生，通常由写入、
对象元数据变更、硬链接或需要 upper 目标的 rename 等操作触发。

## 6. 修改与 copy-up

lower 对象永远不是 Overlay 修改的直接目标。对 lower 对象执行需要写权限的
操作时，Overlay 先确保目标路径的父目录存在于 upper，然后按需要：

1. 在 upper 创建目标对象；
2. 复制必要的对象元数据；
3. 对普通文件复制数据；
4. 复制扩展属性；
5. 在完成准备后将 upper 对象发布到目标名称；
6. 将后续修改路由到 upper 对象。

copy-up 的具体锁、并发、回滚和 workdir 生命周期属于后续 ownership/lifecycle
与 concurrency 阶段。本阶段只确定其语义职责，不冻结实现步骤。

## 7. 持久化和崩溃一致性边界

Overlay 的持久化契约是：

> Overlay 不提供独立的通用事务层，也不承诺强于 upper filesystem 的一般崩溃
> 一致性保证。

Overlay 可以使用临时对象、workdir、底层 rename/link、发布顺序和 fsync 调用，
来避免正常运行时把未完成的 copy-up 作为完整对象公开，并协调 upper 的持久化
过程。但这些手段不构成以下承诺：

- 多个底层操作组成一个全有或全无事务；
- 崩溃后所有相关对象自动回滚；
- 任意崩溃场景都恢复到唯一确定的旧状态或新状态；
- Overlay 提供超越 upper filesystem 能力的 crash consistency。

如果后续纳入 Linux 风格的 fsync mode，则该 mode 应被描述为对 underlying
filesystem 的同步协调策略，而不是 Overlay 自己的事务系统。具体 mode、copy-up
发布顺序和恢复清理策略属于后续范围。

在基础设计中，upper 是修改状态和 Overlay 元数据的持久化权威。若未来纳入
metadata-only copy-up 或 data-only lower，必须明确限定为：upper 仍然拥有元数据
和数据来源关系，实际文件数据可以继续由 lower 提供。

## 8. Stage A 顶层不变量

1. 可见命名空间只能由已配置且有序的 layer stack 推导，不能绕过 Overlay 规则
   直接暴露被隐藏的 lower 对象。
2. 最高优先级的非目录对象是该名称的终止结果；只有兼容的目录对象才能形成
   merged directory。
3. whiteout 和 opaque 是可见性屏障，不是普通用户可见对象。
4. 没有显式合法 upper/workdir 的 mount 不得修改任何 lower 对象。
5. Overlay projection 负责协调多个真实对象，但不成为独立的数据或持久化拥有者。
6. 正常运行时只有完成语义准备的 upper 状态才可以发布为当前可见修改结果；这
   不等同于崩溃后的事务回滚保证。
7. underlying filesystem 负责真实对象、数据、普通元数据和持久化存储；Overlay
   不能承诺超越其能力的通用持久化语义。

## 9. 与 Linux overlayfs 的交叉验证

本阶段模型与 Linux overlayfs 的基础语义一致：

- Linux 文档定义 upper/lower 覆盖和目录合并：[Upper and Lower](/home/ayd/linux/Documentation/filesystems/overlayfs.rst:84)。
- Linux 文档明确说明 merged directory 只合并名称，upper 提供元数据和扩展属性：[Directories](/home/ayd/linux/Documentation/filesystems/overlayfs.rst:109)。
- Linux 文档定义 lower 对象的 copy-up 流程：[Non-directories and copy up](/home/ayd/linux/Documentation/filesystems/overlayfs.rst:267)。
- Linux 实现的 `ovl_fs` 保存 layer stack、workdir 和 mount state，`ovl_entry` 保存 lower stack，`ovl_inode` 保存 upper dentry 与 entry：[ovl_entry.h](/home/ayd/linux/fs/overlayfs/ovl_entry.h:33)。
- Linux 在没有 upper 时将 Overlay 标记为只读，并在 upper 存在时要求 workdir：[super.c](/home/ayd/linux/fs/overlayfs/super.c:1426)。
- Linux 支持 `fsync=volatile/auto/strict`，并在 copy-up 中按策略同步 upper：[params.c](/home/ayd/linux/fs/overlayfs/params.c:144)、[copy_up.c](/home/ayd/linux/fs/overlayfs/copy_up.c:360)。这验证了“Overlay 不提供通用事务”这一边界，同时要求把“完全没有持久化协调”排除在设计表述之外。

Linux-specific 的 inode number、NFS export、redirect_dir、metacopy、verity、
嵌套 xattr escaping 和其他高级选项不在本 Stage A 中展开；它们必须在总体模型
稳定后按与 P0/P1 的耦合程度单独决定是否纳入。

## 10. Stage A 的结束条件

Stage A 在以下模型被接受时结束：

- 命名空间与 projection 已明确区分；
- upper/lower 覆盖、目录合并和可见性屏障已明确；
- Overlay 与 underlying VFS 的语义边界已明确；
- upper 的可选性、可写条件和已有内容语义已明确；
- copy-up 的语义职责已明确；
- Overlay 不提供独立通用事务和超越 upper 的崩溃一致性保证；
- fsync 等持久化协调被保留为后续阶段的具体设计问题，而没有被误写成事务
  承诺。

下一阶段进入 ownership/lifecycle，回答 mount、layer stack、projection、
copy-up state、upper authority 和 workdir 分别由谁拥有，以及这些状态何时
可以发布或销毁。
