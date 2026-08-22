## 附录 A：归并决策（类型与文件）

> 完整符号-文件映射见同目录 `symbol-file-map.md`。

**消灭的非实体类型**（内容内联或改自由函数）：`PositiveBinding`、
`HiddenEvidence`、`BindingKey`、`LookupOutcome`、`WorkdirWorkspace`、
`PreparedTemp`、`PromoteTarget`、`CommitMarker`、`XattrPolicy`（ZST）。

**合并的文件**：`inode/copyup/mod.rs`（仲裁/制备/发布三段）、`readdir.rs`（服务/索引/
`..` 三段）、`inode/inode_cache.rs`（`RealObjectKey` 原在 `lookup/key.rs`，
现归 `real.rs`）。

**改名**：`claims.rs → inuse.rs`（对齐 VFS `OverlayInuseSlot`；类型
`UpperWorkdirClaim → UpperWorkdirInuse`、`InodeClaimGuard → InuseGuard`）；
`probe.rs → capabilities.rs`（以测量产物命名）；`binding/ → lookup/`（缓存消
亡后模块实质即 lookup 操作，`Binding` 更名为 `Lookup`）。

**`RealPath` 字段精简（已确定）**：`RealPath` 不再缓存 `inode`；inode 通过
`Dentry`/`Path` 获取。迁移时需将依赖 `RealPath::inode()` 的调用点改为经
`Path::inode()` 获取。

## 附录 B：讨论点

### D1. BindingCache 的消除与 revalidation 模型

**现状**：当前 `BindingCache` 是 verify-then-serve——每次 lookup 必先重扫层
真相，缓存命中仅复用旧 `Arc`，从不避免层扫描；同时 mutation 要维护全套发
布/失效/对账代码。即“维护可信缓存的全部复杂度 + 零命中的全部运行成本”。

**评审意见**（#3708, 2026-08-21）：缓存不自证（R1）；与 DentryCache 职责重
叠（R2）；单点无法完整支持“绕过 overlay 直改底层”（正 dentry 命中不进
overlay，R3）；Linux 将直改底层定义为 UB，应先文档化预期行为再设计（R4）；
现存消费者均可替代（#3735/#3745 让 mutation 复用 DentryCache 解析，readdir
有 ReaddirIndex，R5）；或改用简单得多的机制（R6）。

**Linux 考证**（`~/linux` 源码）：overlayfs 的 `d_revalidate` 是对底层 fs 自
身 revalidation 需求的**透传**（NFS 式外部变化），且**负 dentry 的
revalidate 标志被显式清除、无条件信任**（`fs/overlayfs/super.c:121` 注释）。
负项不存在“ overlay 自查真相”的机制。

**提案（选项 C：消除）**：

- 删除 `BindingCache` 及其发布/失效/对账全套；`NegativeLookup` 降为无负载三
  变体；保留 `InodeCache`（正确性机制），`object_id` 改惰性计算（缓存命中不
  重读 origin xattr）；
- mutation 的语义维护只剩 ReaddirIndex、物理 whiteout、impure marker；
- 负 dentry 对齐 Linux 无条件可信：删除 `REVALIDATE_ABSENT` 与
  `revalidate_absent`（负查找热路径随之零 fs 调用）；
- 声明“挂载期间绕过 overlay 直改 upper/lower = UB”（对齐 Linux 文档）；
- `stale-upper → ESTALE` 语义**保留**，载体从 BindingCache 改为
  `Once<RealObject>`：Once 有内容即"曾经 upper-backed"，remove 在 fresh
  扫描为 absent 且无 whiteout 时据此前报 `ESTALE`（目标 inode 经 #3745 的
  resolved inode 获得；合入前暂退化 `ENOENT`）；
- 已知边界：正项不透传底层 fs 的 revalidation 需求（如以 procfs 为 lower），
  记录为 documented limitation；若日后实现，形态为透传而非缓存。

**备选（选项 B：锁内可信缓存）**：lookup 在 `DIR` 锁内直接信缓存、不扫层，
mutation 点更新作为正确性支柱——把今天白付的维护成本兑现成真实快路径。需先
完成点更新完整性证明。本文结构对该决策稳健：选 B 仅意味着 `inode/lookup.rs` 多一个
缓存文件，其余不变。

### D2. 未进入功能的未来加入方案

结构对三类已知可选功能的承载力（验证结构不是为现状特制）：

- **redirect_dir：纯增量，零新模块。** 写侧在 `dir/rename.rs`（现
  `cross_device_gate` 的 EXDEV 门即预留位置）；读侧在 `inode/lookup.rs`（扫描
  跟随 redirect 记录，链式跟随加深度上限）；记录名已在 `inode/xattr.rs` 后
  缀表；`redirect=` 模式归 `mount/options.rs` + `fs/policy.rs`。
- **metacopy：零新模块，但触及对象模型。** 本质是权威按方面分裂（upper 持元
  数据、lower 经 redirect 持数据），`RealObjectStack` 需长第二根权威轴，
  `select_real_inode` 按 Data/Metadata 方面选择。触点集中在 `inode/mod.rs`、
  `inode/permission.rs`（元数据写走 metacopy 臂）、`inode/copyup/mod.rs`（制备臂 + 后续全量
  升格）、`inode/data.rs`。
- **persist index（index=on）：一个叶子文件 + 一个 VFS 缺口。** indexdir 是
  第三个排他认领对象（`inuse.rs` 模式复制）；索引机制（origin 编码、探测、
  link 去重、一致性校验）落新叶子 `inode/index.rs`，与最大消费者
  `inode/copyup/mod.rs` 同级；`dir/link.rs` 的 split-inode 退化随之消失。缺口：索引名
  需要底层 file-handle 编码，VFS 可能需原生格式过渡；NFS export 本体列为
  documented limitation。
- **试金石**：`userxattr`（`inode/xattr.rs` 一处前缀表切换）与 `volatile`
  （`inode/copyup/mod.rs` 发布臂一个策略位）应为单点改动；嵌套 overlay 的转义类已事先
  在 `inode/xattr.rs` 就位。

### D3. 其他遗留议题（明确不属于本结构）

- **#3745（unlink/rmdir 传 resolved inode）**：合入后可用于恢复 stale 检测
  （若需要），属独立增强；
- **`RealPath` 字段精简**：`RealPath` 不再缓存 inode，inode 通过 `Path` 获取；迁移见附录 A。
- **CreatorCredential**：未来 VFS 提供凭证 API 后，它是管线的一个阶段加一个
  作用域原语，`inode/permission.rs` 单文件足以支撑，不需要结构位置（详见评审记录
  与 `MountPolicy` 下的 TODO）。

## 附录 C：代办区（迁移执行前必读）

本结构以"逻辑级复用"为原则，但下列点与现有代码冲突，二次重构时必须**改写**
而非搬移。按风险分组，每项给出现状符号（可 grep 核验）。

### T1. 行为语义变化（xfstests 重点观察区）

| #   | 位置（现状符号）                                                                               | 冲突与所需动作                                                                                                                                                                                                                            |
| --- | ---------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A1  | `dir/remove.rs::translate_stale_upper_enoent` + `remove_target` 的 `is_stale_upper` 分支       | **语义保留，载体更换**：stale-upper 检测改由 `Once<RealObject>` 承载（Once 有内容 = 曾经 upper-backed；fresh 扫描 absent 且无 whiteout → `ESTALE`）。目标 inode 经 #3745 resolved inode 获得；合入前暂退化 `ENOENT`（局部开关）。行为不变 |
| A2  | `inode.rs::revalidation_policy_impl` / `revalidate_absent_impl`                                | 两方法删除，策略返回空：负 dentry 从"永不信任"变"永远信任"。行为变化（底层直改后的负名不再可见）                                                                                                                                          |
| A3  | `copyup/promote.rs::publish_via_rename`                                                        | commit 段收编进 publication_parent 的锁 + 新增名字活性复查：升格期间名字被并发删除时中止。原代码无此检查（会复活名字），是新失败路径                                                                                                      |
| A4  | `projection/mod.rs::lookup_binding` 的 verify-then-serve（`matches_truth`/`is_same_negative`） | 删除。lookup 不再比对缓存与真相，纯 resolve                                                                                                                                                                                               |
| A5  | `dir/mod.rs::publish_whiteout_binding` / `publish_positive_binding` / `invalidate_stale_cache` | 语义发布段整体删除；mutation 收尾只剩 ReaddirIndex 维护 + impure 刷新                                                                                                                                                                     |

**已核对（2026-08-22，基于 21 例 xfstests 源码）**：

- A1 是硬依赖：`overlay/012` 的 golden 明确要求 `Stale file handle`，必须保留，只换载体。
- A2 在 21 例中没有“直改底层后期望负 dentry 可见”的硬依赖；`overlay/063`、`overlay/019` 只要求 no-crash，作为回归重点。
- A3 无确定性用例，作为安全加固；`overlay/019` 可能触达该路径。
- A4 无用例要求 verify-then-serve 本身；`overlay/012` 可由 A1 新机制满足。
- A5 需要保留 `overlay/038`、`overlay/077` 依赖的 ReaddirIndex + impure 行为。

### T2. 高危语义点与回撤余量（动工前必须先验）

A1–A4 涉及的语义在既有 xfstests 中疑似有硬性依赖（特别是要求 stale 检测、
以及直接写底层后期望 overlay 可见的用例）。动工前先核验 xfstests 是否确实
要求这些语义：

- **若要求**：对**该点**做小幅回撤，候选回撤形态——
  - A1：（已解决，非回撤）stale 检测改由 Once 承载，见 T1.A1；唯一依赖是
    #3745 的合入时序，合入前退化 `ENOENT`；
  - A2：保留 `REVALIDATE_ABSENT` + 恒 `false`（现状行为，两行开关）；
  - A3/A4：相应调整 commit 复查策略或保留局部比对；
- **不回撤的大面**：BindingCache 的消除、锁形态、模块结构——高危点全部是
  局部开关，不构成结构前提；
- 每项高危点合并前跑 xfstests 基线、合并后对比，差异必须可解释。

**已核对（2026-08-22）**：21 例中只有 A1 被 `overlay/012` 硬性要求；A2/A3/A4
没有显式 golden 依赖。回撤开关保留；合入后重点回归 `overlay/012`、
`overlay/019`、`overlay/063`。

### T3. 状态模型重写（字段级，非搬移）

| #   | 位置                                                            | 冲突与所需动作                                                                                                                                             |
| --- | --------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| B1  | `inode.rs::OverlayInode.facts` + 全树 `facts_snapshot()` 调用点 | `Mutex<RealObjectStack>` → 不可变字段 + `Once<RealObject>`；快照克隆点改借用                                                                                   |
| B2  | `inode.rs::OverlayInode.key` 及 `key()` 读点                    | 字段删除，派生自可见源；`replace_facts` 双锁分步写重写为"alias 装好 → Once.set"                                                                            |
| B3  | `dir_transaction_lock` + `readdir_index` 两锁                   | 合并为 `lock: Mutex<Option<ReaddirIndex>>`；全部取锁调用点改写                                                                                             |
| B4  | `inode.rs::append_write`                                        | 从 facts 锁改持新锁                                                                                                                                        |
| B5  | `readdir_index.rs::ensure_readdir_index` 的复核 + `EIO` 分支    | **未裁决**：DIR 排除不了 facts(D) 变更（commit 取的是 DIR(P)）；可删依据只能是"目录升格保持可见序列不变"（upper 空 + lowers 保留）。先核验此不变量再定删留 |
| B6  | `spin::Once` 引入                                               | 确认依赖（kernel 已有，如 comps/time）                                                                                                                     |

**已确认（2026-08-22）**：B1–B4、B6 为确定改写；B5 仍需先验证“目录升格保持
可见序列不变”再定删留；`spin::Once` 依赖已确认可用。

### T4. 调用顺序/协议重排

| #   | 位置                                                   | 冲突与所需动作                                                                                                     |
| --- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------ |
| C1  | `dir/mod.rs` 五条入口 + rename                         | `check_permission` 与取锁顺序对调（先升格后取锁）；入口契约注释改写                                                |
| C2  | `dir/link.rs::link_source` / `dir/rename.rs` 源升格    | 前置到取锁前；rename 源需预解析，靠 A3 活性复查兜底                                                                |
| C3  | `copyup/trigger.rs` + `promote.rs`                     | commit 段移入 publication_parent 的锁内；CUL→lock 唯一锁序边在此形成。死锁风险区：核验"持锁不取 CUL"在全部入口成立 |
| C4  | `projection/mod.rs::project_inode` 的 `object_id` 预算 | 改惰性：缓存命中不再 `read_lower_id`；流程重排                                                                     |

**已确认（2026-08-22）**：C1–C4 为确定改写；C3 的死锁风险需在全部入口核验
“持锁不取 CUL”。

### T5. 类型消灭引发的机械改写

`XattrPolicy`(ZST→自由函数）、`CommitMarker`(→局部 bool)、
`PreparedTemp`/`PromoteTarget`(→参数）、`WorkdirWorkspace`(→
`UpperWorkdirInuse.workspace`)、`LookupOutcome`(→`Lookup`/元组）、
`PositiveBinding`/`HiddenEvidence`/`BindingKey`（内联）、`OverlayInode`
双构造点合一。

**已确认（2026-08-22）**：这些类型在当前代码中全部存在，需按上文消灭；
`OverlayInode` 双构造点为 `new_root` 与 `project_inode`。

### T6. 归并/去重改写

暂存发布骨架 ×4 统一为 token 化 `publish_temp`（`finish_promotion`/
`create_over_whiteout`/`link_over_whiteout`/`clear_empty_exchange`）；属性迁
移两套合一（promote 的 transfer_* 与 clear_empty 内联拷贝）；
`mknod_object_type` 单点化。

**复核补充（2026-08-22，基于当前源码 + PR #3708 comments 斟酌）**：

- ~~promote 四臂再抽 closure 骨架~~：**不采纳**。当前四臂已经处于基本代码复用状态；
  再引入 closure 会复杂化类型，收益不抵成本。
- ~~upper-only 薄 helper~~：**不采纳**。避免为少量重复引入过薄 helper，
  导致代码树过深。
- **workdir Exchange 发布 + displaced 清理**：保留候选，但需先论证确实可复用；
  若实现需要泛型/闭包，则放弃。
- **`publish_whiteout` 的 Replace 两臂合并**：仅当可复用行数足够大才做；
  行数很小则不做。
- **best-effort impure marker 刷新 helper**：采纳。
- **whiteout 发布后的索引收尾 helper**：采纳，但需注意 remove 与 rename 的作用域/
  未来跨设备 rename 可能不同，不能强行合并；先按当前共同子集抽，保留各自差异。
- **`RealObjectStack` 字面量构造统一走构造器**：可采纳，属低风险一致性收敛。
- **workdir root 单一信源**：不作为“删薄封装”处理；应将运行时 workdir root 收敛为
  全局单一权威来源，避免两个入口各自为政。

**PR #3708 reviewer 去重/简化意见核对（2026-08-22，gh api 拉取）**：

已修：

- `MountOptionKey` 已删除；
- `MountLifecycle` / `MountPhase` / `begin_shutdown` 已删除；
- `CreatorCredentialPolicy` 已删除；
- `superblock.root_inode` 缓存已删除；
- `Layer.root_inode` 冗余字段已删除；
- `WorkdirWorkspace.inode` 冗余字段已删除；
- `workdir_temp_serial` / per-mount serial 已删除，workdir 命名已统一为 CSPRNG 随机名。

仍未修 / 仍存在：

- `mount/options.rs` 仍有 3 处重复 `value.is_empty()` 检查（lowerdir/upperdir/workdir）；
- `RealPath.inode` 冗余字段的迁移：改为经 `Path::inode()` 获取；
- `copyup/trigger.rs::ensure_upper_authority_inner` 的 `let _fs = self.fs_arc()?;` pin 仍在，
  reviewer 认为可去掉；
- `projection/binding_cache.rs::PositiveBinding` 的“薄包装”仍在；A5 删除 BindingCache
  时会一并消失。

### T7. 可见性/壳层改写（机械但量大）

`*_impl` 转发壳整层删除（trait 方法直接调族入口）；`OverlayFs` 字段私有化
+ 访问器；全树 `pub(in overlayfs)` 按目标档位逐处收窄。均为编译器可驱动。

**已确认（2026-08-22）**：当前约 43 个 `*_impl` 转发壳、约 125 处
`pub(in overlayfs)`；目标 `fs/`、`inode/` 目录与 `layer.rs`/`real.rs` 单文件
尚未建立，需随模块迁移一并执行。

---

## 附录 D：设计原则（推导备注）

> 本节为结构推导的备注，正文不再依赖本节展开。

结构从六条原则推导，后文所有放置决策都可回溯到这些原则：

1. **双对象脊柱**：一个文件系统实现只有两个核心对象——挂载对象（`OverlayFs`）与
   逻辑对象（`OverlayInode`）。每行代码必须唯一回答“属于哪一个核心对象”。
2. **单向地层**：依赖方向只允许向下——契约 → 行为 → 对象模型 → 底层模块。
   任一层的文件可以在不打开下层文件的情况下读完其接口。
3. **实体命名**：文件与目录以领域实体命名（类型、机制、资源），不以视角或
   动作命名。
4. **`pub` 即契约**：一个模块的公开项恰好是它的契约（拥有的类型 + 入口操
   作）；其余一切私有。可见性层级是结构的结果，不是目标。
5. **一族一文件**：一个行为族默认一个文件；仅当内部存在真实的实体边界（独
   立不变量 + 足够体量 + 独立消费者）时才成目录。反之，摆渡类型、单字段
   wrapper、ZST 服务、参数包一律消灭。
6. **结构自证**：注释只解释“为什么”（不变量、风险、外部依据）。一旦注释开
   始解释“这段代码为什么在这个文件”，就是放置错误的自白。

---

## 附录 E：CopyUpLock 与 InodeLock 的锁序讨论

> 本节记录结构提案锁序的推导、Linux 对照与正确性条件。正文只写结论，细节放这里。

### E.1 Linux 的真实锁序

Linux overlayfs 中：

- `ovl_inode_lock()` 即本提案的 CUL，锁在 `OVL_I(inode)->lock`。
- overlay inode 的 `i_rwsem`（`inode_lock`）即本提案 `OverlayInode.lock` / `dir_transaction_lock` 所对应的逻辑 inode 锁。

Linux 源码中的关键事实：

- `fs/overlayfs/inode.c` 的 lockdep 注释给出合法链：
  `inode->i_rwsem` → `OVL_I(inode)->lock`。
  即对同一个逻辑对象，方向是 **InodeLock → CUL**。
- `ovl_do_remove()` 调用 `ovl_nlink_start(dentry)`，后者内部 `ovl_inode_lock_interruptible(inode)`。
  因此 Linux 的 unlink/rmdir 会拿目标 inode 的 CUL，与 copy-up 在同一个 per-object 锁上串行。
- copy-up 的 commit 段确实会在持有 `CUL(child)` 时 `start_renaming_dentry()` / `lock_rename()`，
  但它锁的是 **workdir 与 upper destdir 这两个 real dentry 的 inode**，不是 overlay 逻辑父目录的 `i_rwsem`。

结论：Linux 没有 `CUL(child) → overlay 逻辑父目录 lock` 这条边。  
proposal 的 `CUL → lock` 是 Asterinas 自己的倒置，不是 Linux 的锁序。

### E.2 Proposal 的 `CUL → lock` 为何“有条件正确”

`CUL → lock` 可以做到无环且正确，但必须同时满足：

1. **全树严格“持 lock 不取 CUL”**
   所有 mutation 都必须先把 copy-up/升格做完，再拿目录 lock。任何一条路径破坏该顺序，
   `CUL → lock` 与 `lock → CUL` 就会成环。
2. **A3 的活性复查是 commit 的必需步骤，不是 best-effort**
   因为 proposal 的 remove 不拿 `CUL(target)`，W 和 R 在同一个对象上并没有用 per-object CUL 串行。
   唯一能挡住“whiteout 后复活”的机制就是 commit 时在 `publication_parent.lock` 下重新确认
   名字仍然可见、且仍对应同一个 lower 对象。
3. **递归 copy-up 不能同时持有多个 CUL，也不能持有祖先 lock 链**
   当前 `ensure_upper_authority_inner` 的“先递归父、再回来拿自己的 CUL”结构必须保持，
   否则会出现嵌套 CUL 或 CUL 跨多层祖先的锁序。

在这些条件下，`remove /anc/par` 与 copy-up 并发时可由 A3 活性复查中止；  
“直接 remove /anc”也受 rmdir 的 merged-view emptiness 前置条件保护——必须先删掉可见子项，
而子项的删除/rename 又回到上述串行或活性复查的覆盖范围。

### E.3 风险与倾向

- `CUL → lock` 是自洽设计，但正确性押在“持 lock 永不取 CUL”这个脆弱不变量上。
  后续加入 nlink 维护、index、metacopy，或某个 mutation 不得不在锁内等 CUL 时，容易死锁。
- 更贴近 Linux 的方向是：`lock(parent) → CUL(child)`，copy-up commit 不持 overlay 逻辑父锁，
  而是用 real upper dir lock，并让 remove/rename 在 `CUL(target)` 上串行。
- 若保留 `CUL → lock`，应把它记录为有意偏离 Linux 的设计，并在代码注释中把上述三条不变量列为硬约束。

