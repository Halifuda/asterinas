<!-- SPDX-License-Identifier: MPL-2.0 -->

# B/C-8：Cross-Module Reconciliation

**状态：** 阶段完成（跨模块整合）；不授权生产实现或测试工作

这不是新的业务模块，而是 B/C 联合设计的整合检查阶段。目标是确认前七个
模块在 owner、生命周期、锁、发布和失败路径上能拼成一条一致的 Overlay
语义链，而不是增加第二套状态系统。

## 1. Owner 与生命周期

| 状态 | 唯一 owner | 生命周期与交接 |
| --- | --- | --- |
| mount、layer stack、upper/workdir claim、credential、UUID/fsid、durability policy | B/C-1 mount/layer runtime | Builder 预备，mount commit 后发布；teardown 等待 pinned 使用者结束。 |
| projection、binding、Overlay inode identity、lookup/revalidation | B/C-2 projection/identity | 由已发布 mount 输入构造；BindingCache 与 inode cache 只复用 carrier，不另建 layer 或身份真相。 |
| 目录可见项、cookie、`Valid`/`NeedsRebuild` | B/C-3 `ReaddirIndex` | 每个 Overlay 目录一个当前 index；增量更新失败时保留已提交 namespace，转为 `NeedsRebuild`。 |
| copy-up coordination、authority transition、real file/page-cache handoff | B/C-4 copy-up/file I/O | winner 持有协调状态至语义发布或失败 reconcile；page cache 仍属于当前 underlying inode。 |
| Overlay-local permission、creator-credential 检查入口、private xattr policy | B/C-5 metadata/security | local check 先于任何 upper/workdir side effect；真实 metadata 仍由 underlying authority 持有。 |
| whiteout、opaque、create/unlink/link/rename 的 namespace publication | B/C-6 mutation | 在 parent `DIR` 一致性域内发布 binding、barrier、identity 和 index 变化；不接管 copy-up 或 identity owner。 |
| `xino`、origin/index、cleanup、metacopy、NFS handle 等可选记录 | 各自的可选 seam，受 B/C-1～B/C-6 约束 | 只扩展既有 projection、authority 或 mount hygiene；不创建第二套 namespace、identity、data 或 page-cache owner。 |

所有跨模块操作只持有 mount/layer 的 pinned 引用，不让 mount runtime 强拥有会反向强拥有 mount 的 inode。RAII 负责运行时引用释放；workdir 残留、private xattr、UUID/index 等持久化修改仍须显式处理，不能把 `Drop` 当成 durable rollback。

## 2. 跨模块不变量

| 不变量 | 统一结论 |
| --- | --- |
| authority | 未完成 physical publication 前 lower 仍是 authority；完成后才把同一个逻辑对象切换到 upper。authority-only copy-up 不重新创建 inode、name binding 或已暴露 cookie。 |
| visibility | workdir temporary、whiteout、opaque 和 private index record 都不是普通 lookup/readdir source；whiteout/opaque 只作为隐藏 barrier 参与 visibility reduction。 |
| publication | 统一流程为 `preflight → underlying physical operation → physical publication → semantic authority/binding publication → ReaddirIndex update 或 NeedsRebuild`。physical success 与 semantic success 不作为两个独立的用户可见成功状态。 |
| cache | namespace-visible mutation 必须同步更新或失效 BindingCache、barrier、identity 和 ReaddirIndex；不发布 partial index。 |
| permission | 先做 Overlay-local read-only/type/credential 检查，再进入 B/C-4 获取 real handle，随后做 underlying creator-credential 检查；local failure 不得产生 upper side effect。 |
| lifetime | 等待、BIO、callback、释放锁后都重新验证 mount lifetime、source identity、authority、binding、barrier、policy 和 index state。 |
| durability | Overlay 的 `DIR` transaction 只是运行时语义一致性域，不是跨对象持久化事务；fsync 和 crash consistency 不超过 upper filesystem 能力。 |

## 3. 锁、BIO 与 callback 边界

全局锁序固定为：

```text
DIR → CUL → INODE → WL → UPPER
```

| 路径 | 规则 |
| --- | --- |
| 普通 lookup/readdir/mutation | 进入同一 Overlay parent `DIR` 并保持到观察、underlying 调用和语义发布完成；普通可 sleep BIO 可以在 sleep-capable domain 内阻塞，不能在 spin lock 内执行。 |
| 多 parent 操作 | 按稳定 object identity 顺序取得多个 `DIR`；同一 parent 只取一次；不允许从下层模块反向获取 `DIR`。 |
| copy-up | 在需要时按 `DIR → CUL → INODE → WL → UPPER` 获取；copy-up winner 将 `CUL` 保持到 physical/semantic publication 或明确失败处理。 |
| cache/upper 辅助状态 | `INODE`、`WL`、`UPPER` 只在其 owner 规定的短区间内持有；`WL` 不跨 BIO、upper VFS call、workdir callback 或等待。 |
| 可能重入的调用 | waiter、unknown callback 或可能重新取得当前 Overlay lock 的 underlying 调用不得持有该 lock；保存 pinned references 和 operation intent，返回后按锁序重新取得并完整 revalidate。普通已证明非重入 BIO 不因此反复释放/reacquire `DIR`。 |
| VFS cache callback | 现有 generic dentry cache guard 与 Overlay `DIR` 是不同 owner；未来 reservation/publication 应在进入 filesystem callback 前释放可重入的 generic guard，再在 reservation 下发布。 |

## 4. 发布、失败与 reconcile

| 场景 | 正常发布 | 失败处理 |
| --- | --- | --- |
| mount commit | Builder 完成 layer/upper/workdir 检查后一次性发布 mount runtime、root projection 和 identity policy。 | commit 前由 RAII 释放运行时资源；workdir cleanup/UUID 写入失败是显式错误，不假设事务回滚。 |
| lookup/readdir | 在 `DIR` 内完成 observation、visibility reduction、identity projection 和 positive/negative publication；index 只发布完整结果。 | stale 或负结果不可证明时重新 lookup/rebuild；partial observation 不进入 cache。 |
| regular-file copy-up | workdir 中完成 full-data、metadata/xattr/origin/durability 准备，随后发布 upper，再切换既有 logical object 的 authority。 | physical publication 前 cleanup 并保留 lower authority；publication 后若语义发布失败，标记 binding/index conservative invalidation 并 reconcile，不承诺通用 rollback。 |
| namespace mutation | upper operation、whiteout/opaque/redirect intent 完成后，在受影响 parent `DIR` 内更新 binding、barrier、identity 和一个或两个 `ReaddirIndex`。 | 已改变的 upper truth 不回报为 Overlay 成功；重新读取并标记 `NeedsRebuild`/revalidation，禁止 lookup 猜测 partial upper。 |
| authority-only transition | 保留已有 inode、binding、name 和 cookie，只更新 upper/lower authority/provenance。 | waiter 只能依据 fresh authority 重试，不创建第二个 inode 或重复 cookie。 |
| teardown/cleanup | 先阻止新操作，等待 pinned operation 结束，再释放 runtime；可选 cleanup 单独处理 workdir/index residue。 | cleanup 失败记录为残留/备选恢复问题，不把残留当作可见 namespace，也不由 `Drop` 隐藏错误。 |

## 5. 已解决冲突与可选链

- “lock-neutral”不是普通 lookup/readdir/mutation 的默认模式；默认保持一个
  Overlay parent `DIR`，只有有证据的重入或反向锁序才释放并重新取得。
- authority-only copy-up 不会重新生成 inode、name 或 cookie；只有 namespace
  真正变化时才更新 `ReaddirIndex`。
- `xino` 是 identity projection；`origin/index` 是 lower-upper 关联；NFS
  handle 是经过验证的 export 编解码。三者相关但不互相替代，顺序保持
  `index → workdir/index cleanup → NFS export write`。
- `metacopy`/data-only lower 仍是可选能力；若启用，upper 必须能持久表达
  metadata-only marker 与独立 data authority，且不能把 workdir temporary 当
  作零大小的可见文件或第二个 cache。基础路径不依赖该能力。
- lower/merged directory 的跨目录 rename 默认在任何 upper side effect 前返回
  `EXDEV`；`redirect_dir` 只有未来显式启用时才改变这一点。
- `traps` 与 fs-verity 排除当前范围；cleanup 只作为备选恢复增强。NFS 是整体
  可选能力，不拆出只读 NFS 阶段。

## 6. 残余风险与非目标

- B/C-3 的 `NeedsRebuild` 和 B/C-6 的 mutation publication 已在 Stage B/C
  收口时完成一致性核对；VFS 接口落点仍属于后续 Stage F 的设计工作。
- Asterinas VFS 对 positive/negative filesystem-private payload、lookup
  reservation/publication 及底层 xattr/ACL/fileattr 能力仍需在 Stage F 定义；
  本稿不冻结 Rust 类型或函数签名。
- upper filesystem 的 rename/exchange、whiteout、private xattr、metacopy 和
  durability 能力必须在 mount/preflight 明确探测；Overlay 不补足 upper 缺失的
  crash consistency。
- 不实现 traps、fs-verity、跨对象通用 rollback、第二套 page cache、`ino → name`
  反向索引，亦不在本阶段调度 Creator/Checker 或运行 xfstests。

## 7. 阶段结论

B/C-1 至 B/C-7 可以在同一套 owner、生命周期、锁序和 publication/reconcile
规则下组合。剩余问题主要是 B/C-3 的形式签收、VFS 接口落点和 Stage D 的可选
能力取舍，不再存在需要另建核心 namespace 或 identity owner 的跨模块冲突。

BC-8 完成后，Stage B/C 的设计整合检查完成；下一步转入 Stage D 的 scope
decision 与综合设计审查，仍不授权生产实现。
