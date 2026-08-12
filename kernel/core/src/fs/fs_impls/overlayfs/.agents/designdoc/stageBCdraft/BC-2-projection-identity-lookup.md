<!-- SPDX-License-Identifier: MPL-2.0 -->

# B/C-2：Projection、Identity、Lookup

**对应 meso-components：**

- `identity_and_carriers`
- `path_lookup_visibility`

**设计主题：**

- Overlay projection 如何表达 upper 对象和 lower 对象集合；
- 真实对象引用与 layer stack 生命周期的关系；
- lookup 如何发布非目录、目录合并、隐藏和失败结果；
- projection 的缓存、失效和 revalidation；
- 对象身份、来源 layer 和可见 namespace 的一致性；
- lookup 期间的并发读取和 underlying callback 边界。

#### 17. B/C-2 的边界与基本术语

B/C-2 负责把 B/C-1 发布的 mount snapshot 投影为 VFS 可以使用的 root、
binding 和 inode carrier。这里的“投影”不是再创建一套 layer，也不是把
underlying inode 直接暴露给 VFS，而是根据当前 mount 的可见性规则，为一个
Overlay namespace 生成稳定、可复用的中间表示。

本阶段只消费 B/C-1 已经发布的内容：layer stack、真实 root 引用、mount
lifetime、writable/upper policy 和 identity policy。它不重新解析 mount
options，也不负责 copy-up、whiteout 写入、目录合并 readdir 或持久化策略。

这里的 lifetime pin 表示对真实 root、layer mount 和相关 filesystem context
持有强引用，保证 binding 或 inode carrier 存活期间 underlying 对象不会被
释放。pin 只保证生命周期，不保证对象的 whiteout、opaque 或可见性状态永远
不变；后者由本阶段的锁、cache 更新和 revalidation 保证。

#### 18. 三种 projection

三种 projection 在概念上是不同的责任，在一次普通 lookup 中可以连续发生；
它们不是三个独立的 syscall 或三套 lookup。root projection 发生在 mount
发布阶段，name projection 和 ID projection 通常发生在同一个 name lookup
流程中。

##### 18.1 Root projection

root projection 消费 B/C-1 已发布的真实 root 引用和 identity policy，生成：

- 一个 root binding，描述 Overlay root 与真实 root 的关系；
- 一个 root `OverlayInode`，作为 VFS root inode 的 identity carrier；
- 最终供 VFS 使用的 root dentry/path。

它发生在 mount 流程的 commit/publication 阶段，不通过普通的
`(parent, name)` lookup，也不需要先获取 Overlay parent `DIR` 锁。root
发布成功后，root binding、root inode carrier 和 B/C-1 的 pinned lifetime
一起受 mount lifetime 管理；发布失败则按 B/C-1 的逆序回滚。

##### 18.2 Name projection

name projection 的输入是一个已解析的 Overlay parent 和一个 name。它按照
upper-first 的顺序观察各 layer，并进行 visibility reduction：

1. 先观察 upper 中该 name 的状态；
2. 若 upper 是 whiteout，则该 name 被隐藏，停止向 lower 继续寻找；
3. 若 upper directory 是 opaque，则其目录下未在 upper 中出现的 name 不再
   从 lower 继承；
4. 否则继续按 layer 顺序观察 lower；
5. 对可见对象区分 single object 和需要后续合并的 directory object。

概念结果如下：

```text
BindingResult =
    Positive(PositiveBinding)
  | Negative(NegativeBinding)

PositiveBinding =
    Single(single real binding)
  | Merged(merged directory inputs)

NegativeBinding =
    Absent
  | HiddenByWhiteout(whiteout evidence)
  | HiddenByOpaque(opaque evidence)
```

`Single` 表示可见对象只由一个 real object 承载；`Merged` 表示该 name
对应一个可见的合并目录，其后续目录项合并由 B/C-3 负责。`Absent` 表示在
当前可见 layer 集合中没有对象；`HiddenByWhiteout` 和 `HiddenByOpaque`
表示对象可能存在于 lower，但被 upper 的遮蔽规则隐藏。三种 negative
结果对 VFS 都表现为不可见的 name lookup 失败，不应为 hidden object 伪造
一个 inode。

whiteout 和 opaque 是 visibility barrier/evidence，而不是一个可见的
underlying inode。private binding state 可以记录 selected layer、real
binding、whiteout/opaque reason 以及 projection result，从而让后续失效、
unlink、rename 和 copy-up 知道该 name 为什么不可见；但这些状态不应通过
VFS dentry 的 inode 字段向用户空间暴露。

##### 18.3 ID projection

ID projection 只对 positive name projection 的结果执行。它以 mount/layer
上下文、real-object-qualified `OverlayObjectKey` 和 B/C-1 发布的 identity
policy 为输入，生成或复用：

- 一个 Overlay inode identity；
- 对应的 `st_dev`/`st_ino` carrier；
- 一个供 VFS inode 使用的 `OverlayInode`。

ID projection 不重新执行 name lookup、不扫描目录，也不提供 `ID -> name`
的反向映射。一个 real object 可能有多个 hard-link binding，因此多个
`(parent, name)` binding 可以共享同一个 `OverlayInode`；反过来，一个
merged directory binding 仍然可以有自己的 Overlay inode identity。故意不
把“每个 binding 创建一个 inode”作为设计约束。

`st_dev` 和 `st_ino` 是 Overlay namespace 对外发布的身份字段，不应直接
等同于某一层 underlying inode 的裸编号。它们来自 identity policy 对
Overlay object key 的投影；underlying inode 的设备号和 inode 号可以参与
key，但不能单独承担跨 layer、跨 mount 的唯一性。

#### 19. Binding、inode 与 cache

binding 是名字作用域的关系，至少关联 Overlay parent、name、projection
result 和 underlying real binding；inode 是逻辑对象的身份 carrier。两者
生命周期和复用条件不同：同一个 inode 可以被多个 hard-link binding
引用，而一个 binding 的可见来源也可能因 whiteout、opaque 或 mutation 而
失效。

建议使用如下概念性分类，而不在 B/C-2 过早冻结具体 Rust enum 的字段：

```text
positive binding = single | merged
negative binding = absent | hidden-by-whiteout | hidden-by-opaque
```

这可以在实现层落成一个外层 `Positive/Negative` 加内层分类的 algebraic
model。无论最终采用一个 enum 还是多个 enum，都必须保留以下语义区分：

- positive result 能继续进入 ID projection，并可发布可见 inode；
- negative result 不创建 inode，也不把 whiteout/opaque 当作 inode；
- hidden result 要保留足够的 private evidence 以支持失效和后续 mutation；
- `Single` 与 `Merged` 必须区分，因为 merged directory 的后续操作不同。

缓存分为三个互补层次：

1. VFS dentry cache 保存 VFS 可见的 positive dentry，以及 lookup 失败时的
   negative dentry；
2. Overlay private binding cache 保存 `(parent, name)` 的 projection
   result 和 whiteout/opaque evidence；
3. inode cache 保存逻辑对象 identity carrier 的复用关系，key 是完整的
   Overlay identity/object key，而不是裸 `st_ino`。

private binding cache 不是第二份 layer registry，也不是第二份 identity
table。它只保存 name projection 所需的可见性事实。inode cache 也不能被
理解成可靠的 `st_ino -> name` 表：普通 filesystem 中通过 inode 找回名字
通常只是方便的目录反向遍历，而 overlay 中一个 inode 可能有多个名字，且
lower 名字可能被 whiteout/opaque 隐藏。因此 B/C-2 不承诺 ID 到 name 的
反向映射。

#### 20. 一次 lookup 的投影流程和锁边界

cache miss 时，一次普通 name lookup 的概念流程为：

```text
Overlay parent DIR
    -> optional UPPER protection
    -> underlying layer observations
    -> upper-first visibility reduction
    -> positive result: ID projection
    -> binding-cache update
    -> visible VFS result publication
    -> release UPPER
    -> release Overlay parent DIR
```

Overlay parent `DIR` 锁覆盖整个 lookup，并覆盖必要的普通 BIO/sleep；它
保护同一 Overlay directory 内 lookup 与本阶段可观察的 visibility state 的
一致性。UPPER 锁在 `DIR` 之后取得，仅在需要保护 upper visibility state
时取得，并在发布/更新完成后释放。underlying real-parent 的锁由
underlying helper 按其自身规则管理。

这不是要求每个 layer 都被 Overlay 以同一种锁包住，也不是 lock-neutral
lookup 的默认流程。lock-neutral 只在已经证明存在同步重入或反向锁序风险
时作为局部策略；普通 lookup 不应为了它反复拿锁、放锁并重新扫描。B/C-2
不获取后续 mutation 模块的 `CUL`、`INODE` 或 `WL` 锁，也不在本阶段执行
copy-up/whiteout mutation。

underlying callback 的边界必须明确：Overlay 可以在持有 Overlay `DIR`
时调用允许阻塞的 underlying lookup，但不能把一个会回调 Overlay 同一目录
并重新取得 `DIR` 的 callback 设计成隐式重入。需要特殊 lock-neutral 或
retry 语义时，应作为后续实现约束单独证明，而不是由本节默认承担。

#### 21. Whiteout、opaque 和缓存失效

lookup 本身只观察 whiteout/opaque，不改变它们。whiteout 主要在以下
mutation 中发生变化：

- unlink/rmdir lower-only 或 merged object 时，在 upper 创建 whiteout；
- rename lower-origin 或 merged object 时，在源位置创建 whiteout；
- create、mkdir、link 或 copy-up 覆盖一个被 whiteout 的 name 时，删除、
  替换或消耗该 whiteout；
- rename 覆盖目标时，目标位置的 whiteout 可能被消耗或替换；
- opaque 是目录级 barrier，目录级 mutation 可能创建或改变它，但它与
  单名 whiteout 分开维护。

这些变化成功提交后，mutation 模块必须按受影响的具体 `(parent, name)`
更新或失效 binding cache；受影响目录的 opaque 状态也必须一并失效。pin
不能替代这一步，因为 pin 只保护对象生命周期。若某次 mutation 无法证明
private state 仍然有效，则宁可使其失效并重新 lookup，也不能继续使用旧的
positive/negative 结论。

#### 22. VFS negative dentry 与 absent revalidation

VFS dentry 不是 Overlay binding 的完整承载体。正 lookup 可以让 VFS dentry
指向 Overlay inode；失败 lookup 则可以让 VFS 缓存一个没有 inode 的
negative dentry。whiteout/opaque 不应通过给 negative dentry 填入伪造 inode
来表达，否则会把本应不可见的 lower object 重新变成 VFS 可见对象。

Overlay directory 应启用 VFS 的 `REVALIDATE_ABSENT` 策略，并实现
`revalidate_absent(name)`：

- 对 negative cache hit，若 private binding state 仍可证明可信，返回
  `true`，VFS 继续把该 name 视为 absent；
- 若状态已失效、观察不完整或无法证明可信，返回 `false`，VFS 丢弃该
  negative dentry 并重新进入 Overlay lookup；
- 最简单且正确的 baseline 是对 negative hit 直接返回 `false`，每次重新
  检查 upper/lower；
- callback 必须轻量，不能在 VFS children-cache guard 下执行 underlying
  I/O。需要 I/O 的检查应放回正常 lookup 路径。

因此，B/C-2 不要求修改 VFS，也不要求 VFS dentry 携带 Overlay 私有
whiteout/opaque payload。Overlay private binding cache 加上
`REVALIDATE_ABSENT` 已足以表达“VFS 看见 absent，但 Overlay 仍记得它是
whiteout/opaque 隐藏”的状态分离。

#### 23. Version 与最小正确性基线

version 不是 whiteout 正确性的前提，也不是 B/C-2 的必需全局状态。维护
跨多 layer 的全局 version 会增加 mutation、lookup、readdir 之间的耦合；
本阶段采用更小的正确性基线：

- Overlay mutation 通过 parent `DIR` 串行化；
- 成功 mutation 对具体 binding/opaque state 做定点更新或失效；
- 无法证明 cache 结论有效时，通过 `revalidate_absent` 触发重新 lookup；
- pin 只维持真实对象和 layer lifetime。

未来可以引入 per-directory、per-name 或 operation-local validation token，
用于减少重复 underlying lookup，或支持局部 lock-neutral retry；但 token
只是性能优化。任何 token 失效、缺失或无法跨 mutation 证明时，都必须回退
到失效和重新 lookup，而不能把它当作可见性正确性的唯一保障。

#### 24. B/C-2 输出与后续消费

B/C-2 对后续模块输出以下稳定语义：

- mount commit 可发布的 root binding 和 root inode carrier；
- name projection 的 positive/negative result algebra；
- positive binding 到 Overlay inode identity 的复用规则；
- Overlay private binding cache 的 ownership、失效边界和 revalidation
  入口；
- lookup 的基本锁序：`DIR -> optional UPPER -> underlying observation`；
- whiteout/opaque 只作为 visibility state，不作为可见 inode。

B/C-3 消费 `Merged` directory 的目录输入并定义 readdir cache；后续
B/C-4、B/C-5、B/C-6 分别继续消费 mutation 所需的 upper/workdir、inode
操作和 durability 约束。B/C-2 不提前定义这些模块的 copy-up、rename、
unlink 或持久化实现。
