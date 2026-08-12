<!-- SPDX-License-Identifier: MPL-2.0 -->

# B/C-6：Directory Mutation、Whiteout

**状态：** 设计完成，已阶段签收

**对应 meso-component：** `directory_mutations_whiteouts`

**基础 Micro-feature：**

- `P1-21`：upper-only directory create；
- `P1-22`：create over whiteout；
- `P1-23`：create-object dispatcher、whiteout/opaque 分支；
- `P1-24`：create、mkdir、mknod、symlink；
- `P1-25`：whiteout creation；
- `P1-26`：unlink；
- `P1-27`：rmdir；
- `P1-28`：hardlink；
- `P1-29`：same-directory non-directory rename；
- `P1-30`：cross-directory rename 与默认 `EXDEV`；
- `P1-36`：mount-scoped shared whiteout cache。

**条件扩展：** `P2-02` `redirect_dir`。本阶段记录它与 directory rename
的交接和失败边界，但不在基础 Stage D scope 中默认启用。没有明确 redirect
policy 时，lower-backed 或 merged directory 的跨目录 rename 在任何
copy-up、redirect xattr 或 upper namespace side effect 之前返回 `EXDEV`。

## 56. 总体语义：一次 Overlay namespace mutation

Overlay 的目录 mutation 不是对某个 underlying dentry 的简单转发，而是一次
把当前可见 namespace 变化转换为 upper namespace 变化的语义操作。Overlay
首先根据 B/C-2 的 projection 确认 source、target、parent、object kind 和
whiteout/opaque 状态；然后根据 B/C-1 的 mount policy 判断 mount 是否 writable、
upper/workdir 是否仍然有效；再由 B/C-5 完成当前 task credential 下的
Overlay-local permission check。只有这些检查都通过后，操作才可以请求 B/C-4
准备 upper parent、directory promotion 或 lower-to-upper authority transition。

最终的 create、unlink、rmdir、link 或 rename 由 upper filesystem 执行，但
upper physical result 不能直接等同于 Overlay semantic result。成功时，B/C-2
的 binding、object authority、identity/provenance，B/C-3 的 barrier 和
ReaddirIndex，以及必要的 private metadata state，都必须在同一 Overlay
directory consistency domain 内更新或失效，然后才释放受影响的 parent `DIR`。
失败时不能发布 partial binding、partial index 或错误的 lower fallback；如果
upper physical state 已经改变，则进入 conservative revalidation/reconcile，
而不是承诺 Overlay 自己能够进行通用的多对象 rollback。

这里的 transaction 是由 Overlay `DIR` domain 保护的语义 consistency
transaction，不是 Overlay 提供的持久化事务。真实目录项、whiteout、opaque、
redirect 和其它 upper metadata 的 durable truth 始终属于 upper filesystem。

## 57. Workdir：私有 staging，而不是第二个 namespace

workdir 是 upper 同一文件系统上的私有 staging 区。它的作用是先准备一个
temporary object、whiteout 或 upper-side replacement，再通过同文件系统的
rename 或 exchange 将完整结果发布到 upper。workdir 的存在不意味着每个
directory mutation 都必须经过它：普通 upper-only create、pure-upper unlink
和已经 upper-authoritative 的普通 rename 可以直接作用于 upper。

workdir 在以下几类 mutation 中具有实际作用：

1. lower 或 merged parent 需要 promotion 时，B/C-4 可以在 workdir 准备 upper
   directory，再把它发布到 upper parent；这只复制 directory metadata 和
   authority，不复制 lower children；
2. lower regular file、hardlink source 或需要 upper authority 的 rename source
   需要 copy-up 时，B/C-4 在 workdir 准备 private object，完成 data、metadata、
   eligible xattr、origin 和 durability 处理后，再发布到 upper；
3. create-over-whiteout 时，新 object 先在 workdir 中准备，随后替换 upper
   whiteout，避免暴露“whiteout 已删除但新 object 尚未完成”的中间状态；
4. lower-backed unlink/rmdir 或 rename source cleanup 需要 whiteout 时，
   B/C-6 使用由 workdir 提供的 temporary 或 shared whiteout resource，再将
   whiteout 发布到目标 upper parent；
5. 某些 upper directory replacement 或 clear-empty 路径需要先创建一个
   opaque temporary directory，再与 upper object exchange，完成后清理旧的
   upper object。

因此 workdir temporary 在发布前不是 lookup、readdir 或 ReaddirIndex 的
source，也不产生一个对用户可见的 Overlay inode。temporary 的 cleanup 是显式
的、可能失败的操作；workdir 中的残留不能被当作 mutation 成功，也不能被
普通 lookup 当作 upper namespace。B/C-1 负责 workdir 与 upper 的同文件系统
约束、mount lifetime 和 exclusivity；B/C-4 负责 copy-up staging；B/C-6 只
决定何时需要 staging 并消费其结果。

`P1-36` 的 shared whiteout cache 也是 workdir 相关的实现资源。它可以缓存
一个位于 workdir 的 physical whiteout，供后续 mutation 复制、link 或移动到
目标 upper parent；cache slot 本身不是某个 `(parent, name)` 的 namespace
owner，也不是已经发布的 durable whiteout。它的短暂访问由 `WL` 保护，但
`WL` 不能跨 BIO、upper VFS call、workdir callback 或等待。whiteout 一旦发布，
它的语义属于 upper namespace 和 B/C-2 hidden binding，而不再属于 cache slot。

## 58. 统一的 mutation 流程

所有 mutation 共享同一个外层流程，具体操作只在 upper recipe 和可见性结果
上有所不同。

首先，操作在相关 parent `DIR` consistency domain 中重新读取当前 projection，
不能继续使用锁外保存的旧 source 或 target observation。若是跨目录 rename，
两个不同 parent 的 `DIR` 必须按照稳定的 owner identity 顺序一次性取得；相同
parent 只取得一次。随后操作确认 mount 仍然存活、upper/workdir 仍然可用、
target/name/object kind 仍然匹配，并判断当前状态属于 pure-upper、
upper-over-lower、lower-only、merged、whiteout-hidden 或真正 absent。

接着执行 B/C-5 的 Overlay-local permission、type、read-only 和 protected-state
检查。local check 失败时不能进入 B/C-4，不能创建 workdir temporary、whiteout
或其它 upper side effect。local check 通过后，B/C-4 根据 operation intent
准备 upper parent 或 target authority；取得 real authority 后，再由 B/C-5
使用 mount-stashed creator credential 执行 underlying check。B/C-6 不保存另
一份 credential 或 real handle，也不重复定义 copy-up。

upper operation 成功后，B/C-6 在释放 `DIR` 前完成 semantic publication：

- B/C-2 更新或失效受影响的 positive/negative binding、authority、identity
  和 whiteout/opaque evidence；
- B/C-3 更新受影响目录的 ReaddirIndex，分配或移除 Overlay cookie，或者在
  无法安全增量更新时标记 `NeedsRebuild`；
- private marker 的内存状态与已经验证的 upper record 对齐；
- 只要可见 namespace 改变，就不能把它当成仅仅的 authority-only copy-up；
- 如果可见 name 集合没有改变，copy-up 或 promotion 不重新分配 name cookie。

## 59. 可见状态与 opaque 规则

mutation 的关键不是 upper 是否“有一个 inode”，而是 target 在操作前如何被
Overlay 看见。upper 没有同名 object、lower 有同名 directory 时，该 directory
是 visible lower/merged directory；它应继续可见并合并 lower children。普通
`mkdir` 不能把这样的现有 visible directory 静默替换成一个 opaque upper
directory，通常应按 existing-target 语义返回错误；对 lower/merged directory
做 metadata promotion 也应保留 lower children，因此不创建 opaque。

opaque 只在一个更窄的情形出现：操作开始前，同名 lower directory 已经存在，
但在 Overlay 中处于不可见状态，例如被 whiteout 或等价的 name-level barrier
隐藏；本次操作又在该路径重新物化一个 upper directory。此时必须把新 upper
directory 设为 opaque，继续阻止原来被隐藏的 lower directory children 重新
参与 merge。操作前目标原本 absent，或者 lower/merged directory 当时可见，
都不是创建 opaque 的理由；基础方案不创建 opaque。

这个规则也决定了 create-over-whiteout 的语义：如果请求创建的是 directory，
新 directory 在 workdir 中完成准备时就属于 replacement object，opaque record
必须成为其完整 publication 的一部分。若 upper 不支持所需的 replacement 或
opaque 操作，mutation 返回明确错误并进入 conservative revalidation，不能
先删除 whiteout 再把一个未完成的 directory 暴露出来。

opaque 是 directory-level visibility barrier，whiteout 是 name-level
visibility barrier；两者都不是 Overlay 用户可见 inode。`impure` 只提供
identity/origin 侧的辅助事实，不能被 B/C-6 用来推断 lower name 是否隐藏，
也不能代替 whiteout 或 opaque。

## 60. 创建、删除与链接

### 60.1 create、mkdir、mknod、symlink

当 target name 在 Overlay 中真正 absent，且 upper parent 已经 ready，普通
create、mkdir、mknod 和 symlink 直接在 upper parent 创建新的 upper object，
不需要让 target 先经过 workdir。新 object 的 owner、mode、group、SGID、
timestamp 和未来 ACL/fileattr 语义由 B/C-5 的 metadata pipeline 约束；
upper object 完整成功后才创建 Overlay binding、identity 和新的不复用 cookie。

如果 target 是 whiteout-hidden directory，则这是 create-over-whiteout，而不
是普通 upper-only mkdir：新 directory 需要在 workdir 中准备、设置 opaque，
再替换 whiteout。symlink 创建只创建新的 upper symlink，不跟随或 copy-up
symlink referent；symlink 自身后续的 metadata mutation 仍然经过 B/C-4/B/C-5
的既定边界。

### 60.2 unlink、rmdir 与 whiteout

pure-upper object 的 unlink/rmdir 直接删除 upper object，通常不创建
whiteout。如果 upper object 覆盖 lower object，删除 upper 后会使 lower
name 重新出现，因此 mutation 必须在 upper parent 建立 whiteout；如果 target
是 lower-only，则不修改 lower，只在 upper parent 发布 whiteout。whiteout
可以是 upper capability 支持的 character-device marker 或 private xattr form，
具体 physical representation 由 metadata policy 和 underlying filesystem
决定，不能成为普通 xattr 路径可见的 public object。

rmdir 的 emptiness 必须依据 Overlay-visible directory，而不是只扫描 upper
children。B/C-6 消费 B/C-3 的 ReaddirIndex、BindingCache 和 barrier state；
whiteout-hidden children 不算可见 child，但 visible lower、visible upper 或
merged child 都会使 rmdir 失败。如果 index 是 `NeedsRebuild`，必须先在同一
consistency domain 中重建，或者保守地返回错误，不能用不完整的 upper-only
结果宣布 directory empty。

对 lower-backed directory，rmdir 不 copy-up 被删除目录的 children。需要的
upper side effect 是 parent preparation 和 whiteout；如果 upper directory 的
现有 children、whiteout 或 lower fallback 需要先做 atomic clear-empty
replacement，workdir 可以承载 temporary opaque directory，随后 exchange 并
清理旧 upper object。这个 temporary 仍然不是可见 directory source。

### 60.3 link

`link(source, target)` 同时改变 source object 的 hardlink relation 和 target
parent 的 visible namespace。source 先经过 B/C-5 的 local check；如果 source
是 lower-backed，B/C-4 使用 workdir 完成 source copy-up 和 upper authority
publication；然后 B/C-6 在 upper parent 创建指向同一 upper inode 的 hardlink。
target 是 whiteout 时沿用 create-over-whiteout 的 replacement 语义。

本次 link 成功后，source binding、target binding、Overlay identity、visible
nlink/provenance 和 target parent 的 ReaddirIndex 一起更新。没有
`PersistentOriginIndex` 时，不承诺 lower 中原有多个 hardlink alias 在分别
copy-up 后仍共享一个 upper inode；index、export 和更完整的 nlink 语义留给
条件 B/C-7。source copy-up 成功但最终 link 失败时，允许 B/C-4 的 authority
transition 保留，但 B/C-6 不能把 link 报告成成功，且必须按新的 upper truth
更新或失效 projection。

## 61. Rename：upper publication 与默认 `EXDEV`

rename 需要同时保护 source parent、target parent、source binding、target
replacement 和两个 directory index。same-directory non-directory rename
可以先通过 B/C-4 使用 workdir 将 lower-backed source 变成 upper-authoritative，
然后直接在 upper parent 执行 rename。target 若是 pure-upper，按 upper
replacement 处理；target 若有 lower fallback，必须建立相应 hidden state；
target 若是 whiteout，则必须消费或替换 marker，而不能把 whiteout 当普通
rename target。

cross-directory non-directory rename 同样先准备 source authority 和 upper
parents，再在 upper 中执行移动。成功时 source parent 的 source binding、
target parent 的 target binding、target replacement、lower hiding 和两个
ReaddirIndex 必须一起更新。workdir 参与的是 source/parent copy-up 或
replacement staging，最终 visible move 仍然是 upper namespace operation。

基础范围下，lower-backed 或 merged directory 的跨目录 rename 在 policy 判断
阶段直接返回 `EXDEV`。此路径不创建 upper directory，不使用 workdir，不写
redirect，不创建 whiteout/opaque，也不更新 binding 或 ReaddirIndex。pure-upper
directory 的跨目录移动可以执行 upper-side rename，但仍必须通过 target type、
emptiness、permission 和双 parent publication 检查。

如果之后 Stage D 纳入 `P2-02 redirect_dir`，lower/merged directory rename
才可以先由 B/C-4 使用 workdir 完成 directory metadata copy-up，再通过
metadata policy 写入 original path 的 redirect record，最后移动 upper
directory。B/C-2 负责后续 lookup 对 redirect 的解释；redirect 不提供
`ID -> name` 反向映射。redirect policy、path-length cap、follow mode 或
underlying xattr failure 都必须在 publication 前处理，失败时仍不能留下
Overlay 无法解释的 partial move。

## 62. 锁、阻塞与重入

B/C-6 遵循全局锁拓扑：

```text
DIR -> CUL -> INODE -> WL -> UPPER
```

parent `DIR` 是 mutation 的一致性入口；cross-directory rename 的两个
`DIR` 按稳定 owner identity 排序。B/C-6 不从 B/C-4 内部反向获取 `DIR`。
lower-to-upper transition 需要时才取得 `CUL`，authority/identity publication
需要时取得 `INODE`，whiteout cache 只短暂取得 `WL`，upper/workdir access 最后
进入 `UPPER`。同级对象按 `Arc::as_ptr()` 顺序取得，同一实例不能递归获取。

普通、已证明非重入的 upper/workdir BIO、metadata I/O、whiteout 操作和
copy-up staging 可以在允许 sleep 的 domain 内执行；spin lock 不能包住这些
操作。一个可能回调进入 Overlay、重新取得当前 domain、等待当前 copy-up 或
改变同一目录的 callback，则必须走 lock-neutral handoff：保存 pinned
references 和 operation intent，释放可能被重新取得的 Overlay locks，调用
callback，返回后按全局顺序重新取得 locks，并重新验证 source、target、
authority、barrier 和 directory index state。

普通 underlying I/O 不应因此在一个逻辑 mutation 中反复释放和重新取得
parent `DIR`；lock-neutral 只适用于有证据的重入或反向锁序。任何等待、BIO
或 callback 返回都不能让旧的 empty result、whiteout observation、redirect
decision 或 upper handle 直接继续生效。

## 63. 发布、失败与生命周期

在 read-only、missing upper/workdir、invalid name/type、permission failure 或
projection stale 的路径上，不得产生 upper、workdir、whiteout 或 cache side
effect。workdir temporary 在 physical publication 前失败时由 B/C-4 或 B/C-6
显式 cleanup；lower authority 保持有效，waiter 只能从 fresh observation
retry。

如果 physical upper operation 已经完成，但 metadata、whiteout、opaque、
binding、identity 或 ReaddirIndex publication 失败，Overlay 不声称具有通用
rollback。它必须重新读取 upper object type、target binding、marker、origin
和必要的 directory state，把受影响的 binding/index 标为 conservative
invalidated 或 `NeedsRebuild`，再将事实交给后续 reconcile。temporary residue、
shared whiteout cache 和已发布 marker 不能混为一谈；只有 upper namespace
中的已验证 marker 才能影响下一次 visibility reduction。

mutation 持有 mount、layer、source/target binding、upper parent 和 workdir 的
必要 strong/pinned references，直到 operation 完成或明确失败。B/C-1 的
teardown 不能在这些引用仍被使用时回收 layer context；workdir cleanup 是
显式 fallible operation，不以 RAII drop 伪装成 durable rollback。Overlay 不
承诺比 upper filesystem 更强的 crash-consistency，也不承诺多个 upper objects
之间的统一持久化提交。

## 64. 跨模块交接

B/C-6 从 B/C-1 消费已发布的 writable/read-only、upper/workdir lifetime、
creator credential 和 durability policy，不重新解析 mount option 或 claim
第二份 upper/workdir exclusivity。它从 B/C-2 消费当前 binding、identity、
whiteout/opaque evidence 和 revalidation 入口，并在 mutation 后更新或失效
这些 facts；它不创建第二份 identity map，也不提供 `ID -> name`。

它从 B/C-3 消费 merged directory 的 current index、cookie 和 visible emptiness，
并在 namespace mutation 后维护 source/target directory 的 index 或标记
`NeedsRebuild`；它不读取 raw underlying cookie，也不把 workdir temporary 当
作 readdir source。

它从 B/C-4 请求 parent promotion、lower source copy-up、upper handle 和
cleanup/reconcile result，但不接管 copy-up coordination、page cache 或
authority owner。它从 B/C-5 请求 local/underlying permission pipeline 和
private metadata policy，但不保存 real handle、credential 或普通 xattr 内容。
whiteout、opaque 和 redirect 的 namespace spelling、escaping 与持久化仍由
`xattr_namespace_escaping` 及 upper filesystem 负责；B/C-6 只提交清晰的
semantic marker intent。

## 65. 外部验证映射（仅 xfstests）

本阶段沿用已接受的 `directory_mutations_whiteouts` Designer validation
contract。映射是 many-to-many 的外部证据，不表示本阶段已经运行测试，也不
用单个黑盒 case 证明某一个内部 owner、锁或 workdir cleanup 分支。

| Micro-feature | upstream cases | 主要观察 |
| --- | --- | --- |
| `P1-21` | `overlay/013`, `027` | upper-only create 后新名字可见，upper 成为 authority |
| `P1-22` | `overlay/008`, `015` | create-over-whiteout 替换 marker，并保持预期 metadata/SGID 行为 |
| `P1-23` | `overlay/008` | dispatcher 选择正确的 upper-only 或 whiteout replacement 路径 |
| `P1-24` | `overlay/008`, `015`, `016`, `020` | create、mkdir、mknod、symlink operation family |
| `P1-25` | `overlay/006`, `010`, `011`, `031` | lower name 隐藏，whiteout 不从 lookup/readdir/remount 暴露 |
| `P1-26` | `overlay/006`, `011`, `012`, `020`, `031` | pure-upper unlink、lower-backed hide 和 stale dentry 行为 |
| `P1-27` | `overlay/010` | Overlay-visible emptiness、rmdir 和 lower-directory hiding |
| `P1-28` | `overlay/018`, `028` | lower hardlink source copy-up 与 upper link/nlink 结果 |
| `P1-29` | `overlay/032`, `033`, `034` | same-directory non-directory rename、replacement、copy-up/whiteout |
| `P1-30` | `overlay/032`, `033`, `034` | supported move 成功，lower/merged directory 默认 `EXDEV` |
| `P1-36` | `overlay/006`, `010`, `031` | whiteout 可观察；shared cache 复用本身无 isolating coverage |
| 条件 `P2-02` | `overlay/017`, `043`, `057` | redirect rename、original path、lookup/inode 行为 |

本阶段不创建、修改或扩展 ktest、filesystem-local fixture、memory-disk
fixture 或 xfstests harness。未来实现阶段仍必须由 Checker 通过已授权的
upstream xfstests lane 运行并保存实际 evidence。

## 66. 阶段结论

B/C-6 的完整设计结论是：目录 mutation 只改变 upper namespace，不直接改变
lower；workdir 只承担 private staging、whiteout preparation 和必要的 atomic
replacement；whiteout 隐藏一个 lower name，opaque 只在“操作前同名 lower
directory 已存在但在 Overlay 中不可见、随后重新物化 upper directory”时创建；
visible lower/merged directory 和原本 absent 的路径不创建 opaque。所有成功的
namespace mutation 都必须在释放受影响 `DIR` 之前完成 binding、barrier、
identity 和 ReaddirIndex 的一致性发布；所有 physical/semantic 不一致都进入
保守失效和 reconcile。

本阶段已完成设计签收，但不授权 Creator pass、Checker pass、Reviewer、生产
代码、`SYSTEM_BLUEPRINT.md` 或 `PASS_SLICING.md` 更新。
