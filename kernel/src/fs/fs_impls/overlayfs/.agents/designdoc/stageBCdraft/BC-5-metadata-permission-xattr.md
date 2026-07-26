<!-- SPDX-License-Identifier: MPL-2.0 -->

# B/C-5：Metadata、Permission、Xattr

**状态：** 设计完成，已阶段签收

**对应 meso-components：**

- `inode_attributes_security`
- `xattr_namespace_escaping`

**基础 Micro-feature：**

- `P1-16`：`setattr`（chmod/chown/utimes）；
- `P1-17`：`update_time`（atime/mtime/ctime）；
- `P1-18`：两步 permission check；
- `P1-33`：xattr get/set/list delegation。

**条件扩展：** `P2-05` POSIX ACL、`P2-06` fileattr、`P2-13` `userxattr` 和
`P2-14` nested xattr escaping。本文只给出它们如何接入共同检查管线，不代表
已做 Stage D scope decision。symlink、hardlink、origin/index 和 nlink 的
object-kind/copy-up 规则由 B/C-4 统一说明；本阶段只处理这些操作进入权限
管线时的共同检查，不重复定义其对象转换语义。

**前置：** B/C-1 提供 mount read-only 和 stashed creator credential；B/C-2
提供 Overlay object、binding 和 identity projection；B/C-4 提供 real handle、
copy-up trigger、authority transition 和实际 underlying operation 的入口。

## 48. 核心结论：两步检查是一条公共管线

凡是对已投影 Overlay object 执行访问的请求，统一遵循：

```text
Overlay request
    -> Overlay-local permission check
    -> [xattr private/public check, only for xattr operations]
    -> obtain the real handle / current real authority through B/C-4
    -> underlying permission check
    -> delegate the operation to the underlying object
```

这里的两个 permission check 不是两个可选 fast path：

1. **Overlay-local check** 使用当前 task credential，在 Overlay 可见 object 上
   检查 mode、owner/group、ACL（若纳入范围）和本地 MAC/保护规则；
2. **Underlying check** 使用 mount-stashed creator credential，在已经取得的
   real handle 对应的 underlying object 上执行真实 VFS permission/MAC check。

`default_permissions` 只允许省略第二步 underlying delegation；它不省略第一步
Overlay-local check。

两步之间的 real handle 和 current real authority 由 B/C-4 提供。B/C-5 不创建、
缓存或拥有另一个 real handle，也不重新定义 copy-up：

- lower-backed read 请求由 B/C-4 取得 lower real handle；
- lower-backed mutation 请求由 B/C-4 执行既定 copy-up trigger，并取得发布后
  的 upper real handle；
- upper-backed 请求由 B/C-4 取得当前 upper real handle；
- B/C-5 只在第一步通过后请求这条既定入口，然后在 handle 返回后执行第二步；
- 第一步 Overlay check 失败时，B/C-5 直接返回，不获取 real handle，不触发
  copy-up，不产生 workdir 或其它副作用。

因此，本阶段对 B/C-4 的要求只是增加一个明确的调用前置条件：**permission
check 失败不得进入 B/C-4**。它不改变 B/C-4 对 read、write、metadata mutation、
page cache 或 copy-up publication 的既有语义。

## 49. 两步检查的职责边界

### 49.1 Overlay-local check

第一步检查的是“当前 task 是否能通过 Overlay 看到并请求该操作”，而不是
underlying creator 是否能访问真实对象。它必须发生在任何可能的 real-handle
获取或 copy-up side effect 之前。

它至少覆盖：

- 读、写、append、truncate、metadata mutation 和 xattr mutation 所需的
  Overlay mode/owner/group 权限；
- mount read-only 对 mutation 的拒绝；
- 当前 Overlay object 的 type、protected state 和 operation kind；
- 进入 ACL/MAC 扩展后仍属于 Overlay 可见权限的部分。

失败时：

- 不调用 B/C-4 的 real-handle/copy-up 入口；
- 不读取或写入 underlying object；
- 不创建 temporary upper object；
- 返回本地 permission 或 read-only 错误。

### 49.2 Real-handle seam 与 underlying check

第一步成功后，B/C-5 把 operation intent 交给 B/C-4。B/C-4 返回的不是一个
新的 B/C-5 carrier，而是该请求当前可以使用的 real authority/handle。

B/C-5 随后使用 mount-stashed creator credential 对该 handle 做第二步检查。
第二步失败时，B/C-5 不执行最终 underlying operation；对于已经由 B/C-4 完成
的合法 authority transition，仍沿用 B/C-4 的既定 cleanup/reconcile 规则，不能
因为 permission failure 自行发明 rollback。

这意味着 permission failure 的位置不同，责任也不同：

```text
local failure
    -> B/C-5 returns before B/C-4

real failure after handle/transition
    -> B/C-5 returns before final operation
    -> B/C-4 owns any already-started transition cleanup/reconcile
```

如果等待、BIO 或 callback 之后 real authority 可能改变，B/C-4 必须返回经过
重新确认的 handle；B/C-5 不使用过期的 lower/upper observation 继续做第二步
检查。

### 49.3 Credential

- 第一阶段使用 current task credential；它不写入 mount state。
- 第二阶段使用 B/C-1 在 mount 时保存的 creator credential snapshot。
- `override_creds` 只改变 snapshot 的来源，不改变两步检查的职责分工。
- underlying VFS callback 的 credential context 由 B/C-4/underlying seam 管理，
  不由 B/C-5 保存成长期 carrier。

## 50. Xattr 的额外 private/public 检查

xattr 请求仍然遵循两步 permission check，但在第一步之后增加一个仅属于
xattr 的分类阶段：

```text
xattr request
    -> Overlay-local permission check
    -> classify xattr as public or Overlay-private
    -> authorize the selected xattr owner/policy
    -> obtain real handle through B/C-4 when underlying access is needed
    -> underlying permission check
    -> delegate public xattr or private-record operation
```

### 50.1 分类阶段

分类必须在任何普通 underlying xattr delegation 之前完成：

- **public xattr**：普通用户可见的 xattr，按当前 real authority 读取或修改；
  lower-backed mutation 仍交给 B/C-4 处理 copy-up；
- **Overlay-private xattr**：opaque、whiteout、impure、protattr 等由 Overlay
  owner 管理的内部记录；不能被普通 xattr 路径当作 public name 直接透传。
  其它 private record 由其 owner 通过同一 policy dispatch，B/C-5 不定义其
  copy-up、identity 或 link-count 语义；
- **escaped xattr**：nested Overlay 传递时，按 nesting boundary 增减一层
  `overlay.` escape prefix；escaping 只处理名称，不自行改变 visibility；
- **unknown/reserved xattr**：由 policy 判定是否拒绝或按 underlying contract
  处理，不能通过“不是已知 private name”自动变成 public。

`OverlayMetadataPolicy` 的最小职责是 namespace/classification、private owner
dispatch、list filtering 和 nested escaping。它不拥有 ordinary xattr 的持久化
内容，也不替代 B/C-4 的 real handle/copy-up owner。

### 50.2 Private xattr 的第二步检查

private 不是“不需要 underlying permission check”的意思：

- 若 private record 只是在 Overlay 已发布状态中做内存分类，分类本身结束于
  policy authorization；
- 若 private record 需要从 upper/lower 读取或写入，仍必须通过 B/C-4 取得
  real handle，再执行 underlying creator-credential check；
- 普通 xattr set 不能伪造或修改由 lookup、copy-up、whiteout、identity 等
  meso 负责的 private record；
- `listxattr` 必须在返回前过滤 private names；一次 private get/set 成功不等于
  private name 可以被普通列表暴露。

`userxattr` 只改变 private namespace 的选择（默认 `trusted.overlay.*`，开启后
使用 `user.overlay.*`），不能只切换 whiteout 而让 origin/opaque 等记录使用另一
套 policy。

## 51. Metadata 与 atime 的位置

### 51.1 Metadata mutation

`setattr`、显式 timestamp mutation、ACL set、fileattr set 和 lower-backed
xattr set 都是 mutation：

```text
Overlay-local check
    -> B/C-4 copy-up/real-handle seam
    -> underlying creator-credential check
    -> underlying metadata operation
```

B/C-5 只定义检查、metadata policy 和 private-record 分类；B/C-4 仍负责 lower
到 upper 的 transition、publication、temporary cleanup 和最终 handle。

### 51.2 atime 的特殊性

read 造成的 atime side effect 不应被建模成“为了更新 atime 而 copy-up”：

- lower-only read 不触发 copy-up；按 Linux OverlayFS 兼容方向，不更新 lower
  atime；
- upper-backed read 可以在已有 upper authority 上更新 upper atime；
- `ro` mount 或 `noatime` 等 policy 可以跳过该 best-effort 更新；
- mtime/ctime 的显式修改仍是普通 metadata mutation，走两步检查和 B/C-4 seam。

这条规则是两步检查管线的一个特殊 operation policy，不是第三种 permission
check，也不改变 B/C-4 的 copy-up 语义。

## 52. Lock 与失败边界

本阶段不新增 lock domain：

- Overlay-local permission 和 xattr classification 尽量从 `NONE` 进入；
- B/C-4 的 real-handle/copy-up seam 遵循既有 `DIR -> CUL -> INODE -> WL ->
  UPPER` 拓扑；
- B/C-5 不获取 `DIR` 或 `WL`，不持有 Overlay lock 跨越未经证明的
  underlying permission/MAC/xattr callback；
- local check 失败没有 transition cleanup；real check 失败则保留 B/C-4 对
  已开始 transition 的 cleanup/reconcile ownership；
- 不在 spin lock 内执行 BIO、sleep 或 yielding lock。

## 53. 跨模块交接

| 交接方 | B/C-5 依赖 | B/C-5 负责 | B/C-5 不负责 |
| --- | --- | --- | --- |
| B/C-1 | read-only、creator credential、permission policy | 两步检查使用的 credential 角色 | 不重新声明 mount lifetime |
| B/C-2 | object/binding/identity projection | 在已发布 Overlay object 上做 local check | 不获取 real handle，不做 ID 映射 |
| B/C-4 | real handle、copy-up、authority transition | 在第一步通过后调用其既定入口，并执行第二步 | 不改变 copy-up、page-cache、publication 语义 |
| B/C-6 | namespace mutation 与 whiteout/opaque owner | 解释 private xattr 对普通 operation 的限制 | 不直接执行 create/unlink/rename |
| B/C-7/B/C-8 | future identity/data/reconcile contracts | 提供 private policy 的接入点 | 不提前实现条件 feature 或 rollback |

## 54. 初步验证映射（仅 xfstests）

| Micro-feature | 现有映射 | 主要观察 |
| --- | --- | --- |
| `P1-16` | `overlay/025` | lower-backed setattr 经 copy-up 后的 metadata 结果 |
| `P1-17` | 无 packaged mapping | update-time/lower-atime 作为明确 coverage gap |
| `P1-18` | `overlay/015`、`016`、`078` | permission policy 与 combined permission behavior |
| `P1-33` | `overlay/009` | xattr delegation；private filtering 未隔离 |
| `P2-05` | 无 packaged mapping | ACL 作为 coverage gap |
| `P2-06` | `overlay/030`、`075`、`076` | protected fileattr 与属性操作 |
| `P2-13`/`P2-14` | `overlay/083`、`084`、`109` | 当前 packaged lane 未选，保持 unavailable |

本阶段不创建或修改 ktest、filesystem-local fixture 或 xfstests surface。

## 55. 下一轮讨论问题

1. private xattr 的 authorization 是否统一落在 `OverlayMetadataPolicy`，还是
   由各 private-record owner 做最终拒绝；
2. `default_permissions` 下第二步如何在 Asterinas 中显式表现，而不绕过
   private xattr classification。

本稿仍是设计讨论材料，不授权 Creator pass、Checker pass、生产代码或
`SYSTEM_BLUEPRINT.md`/`PASS_SLICING.md` 更新。
