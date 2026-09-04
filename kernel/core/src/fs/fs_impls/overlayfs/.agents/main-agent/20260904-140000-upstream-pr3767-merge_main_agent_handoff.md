<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: 2026-09-04 上游 PR #3767 合并轮（开题）

**Date / Time:** 2026-09-04 14:00 CST
**Status:** `OPEN — 仅开题。user 指令（2026-09-04）：为 upstream PR #3767 的合并问题开新 handoff，先用 gh api 下载 PR comments 放入本文。前手off
20260904-130000-placement-redundancy 保持打开（user 指令，两 handoff 并行）。未经 user 指令不派发任何 packet、不改任何代码。`
**Parent:** 前一 handoff
`20260818-0846-upstream-inode-trait-排期（见
20260830-203000-unit-and-new-regression-tests §18 排队项「Inode trait 重构
PR 的 pick 与适配」——本 PR 即该项的目标）与
20260904-130000-placement-redundancy（并行打开中）。

## 1. PR 元数据（gh api 实取，2026-09-04）

- **PR**: [asterinas/asterinas#3767](https://github.com/asterinas/asterinas/pull/3767)
  **"Pass `Dentry` to some `Inode` trait methods"**
- 作者 cchanging；分支 `cchanging:inode_accept_dentry` → `asterinas:main`
- **State**: open，`mergeable_state: blocked`
- **规模**: 28 changed files，+805/−418；创建于 2026-08-31
- **主题**: 一批 `Inode` trait 方法签名增加 `&Dentry` 参数（metadata
  setters、xattr ops、resize、fallocate、目录增删改 create/unlink/rename
  等），VFS `Path`/`Dentry` 层负责把 dentry 传进 inode 操作；
  `clear_file_priv` 改为经 `Dentry` 工作。

## 2. PR 评论全文（gh api 下载，逐字）

### 2.1 会话评论（issues/3767/comments，1 条）

**lrh2000 @ 2026-09-01T02:52:14Z**（引用 cchanging 的原问题："I'm
wondering whether we could instead restrict the types returned by the VFS
layer, so that upper layers cannot obtain a fully capable `Inode` trait
object. For example, provide a subset of the Inode trait called
`InodePublic`, and ensure that `Path` and `InodeHandle` can only expose
`InodePublic`."）：

> I think this is okay. I don't have any better alternatives at the moment.
>
> It's fine as long as we don't need to actually store an `Arc<dyn Inode>`
> outside `crate::fs`. It seems, however, that there are no such use cases.
> We will store `Arc<dyn FileLike>` or `Path`, etc. I am not 100% sure
> whether this case will never happen in the future[^1], though. Even if it
> happens, a wrapper type may also help (`InodeWrapper(Arc<dyn Inode>)`
> which only exports access to `&dyn InodePublic`).
>
> [^1]: However, storing an `Arc<dyn Inode>` without keeping its
> filesystem/mount alive will likely cause other problems?

**解读**：维护者认可 `InodePublic` 子 trait 方向但未定稿——trait 面可能
还会演化，pick 时点与形态存在变数。

### 2.2 行内评论（pulls/3767/comments，2 条，均为 Copilot）

1. **on `kernel/core/src/fs/vfs/path/mod.rs:7`**（@ 2026-08-31T12:43:54Z）:
   `Dentry` is re-exported as `pub(crate)`, which makes this VFS-internal
   type accessible from anywhere in the `kernel/core` crate and weakens the
   intended layering (especially if the goal is to keep `Inode` operations
   callable primarily via the VFS). All current uses appear to be within
   `crate::fs`, so this can likely remain scoped to `crate::fs` as before.
2. **on `kernel/core/src/fs/vfs/path/dentry.rs:133`**（@
   2026-08-31T12:43:55Z）: Making `Dentry` itself `pub(crate)` exposes a
   low-level VFS structure to the entire crate. If there are no callers
   outside `crate::fs` (which seems to be the case), keeping it
   `pub(in crate::fs)` better preserves encapsulation and supports
   enforcing "call through VFS" expectations.

**解读**：`Dentry` 可见性是本 PR 的敏感点——行内意见建议收紧到
`pub(in crate::fs)`。**对本仓直接相关**：我们的 overlayfs 改造恰好大量
消费 `Dentry`（`RealObject` dentry 锚定、`Layer` clone view 等），若上游
最终收紧可见性，overlayfs 作为 `crate::fs` 子树不受影响；但任何
`pub(crate)` 依赖不能再假设。

### 2.3 评审摘要（pulls/3767/reviews，Copilot bot 摘要，供合并盘点）

变更面（28 文件，Copilot 分文件表）：
- **VFS 核心**：`fs/vfs/path/mod.rs`（re-export + Path 更新，dentry 传参
  wrapper）、`fs/vfs/path/dentry.rs`（可见性 + link/unlink/rmdir/rename
  改走 dentry）、`fs/vfs/fs_apis/xattr.rs`（`clear_file_priv` 改经
  Dentry）、`fs/vfs/fs_apis/inode.rs`（**trait 签名主战场**：resize/
  set_mode/create/unlink/rename/fallocate/xattr ops 增 `&Dentry`）、
  `fs/file/inode_handle.rs`（open/write/fallocate 路径传 Dentry）。
- **默认 impl/各 fs 适配**：systree_inode、anon_pipe、virtiofs、sysfs、
  ramfs（memfd/fs.rs，atime 无 Dentry 可用处的处理）、pseudofs/nsfs、
  procfs template（sym/mod/file/dir）、exfat、devtmpfs、devpts
  （slave/ptmx/mod）、configfs、cgroupfs（rmdir 从 Dentry 取子名）、
  ext2（impl_for_vfs/inode.rs + test）。
- **overlayfs**：`fs/fs_impls/overlayfs/fs.rs`（上游旧实现适配）。

原始 JSON 归档：`/tmp/pr3767/{issue_comments,reviews,review_comments}.json`
（临时区；如需入库可落 components/）。

## 3. 与本仓的合并问题初判（主代理，待 Designer 核实）

1. **overlayfs hunks 不可用**：PR 改的是上游旧版
   `fs_impls/overlayfs/fs.rs`；本仓已整体重构（`overlayfs/mod.rs` +
   `fs/`、`inode/` 子树，legacy 已删除）。PR 的 overlayfs 文件必然
   conflict/不适用——**适配工作 = 按新 trait 签名改本仓 overlayfs 的
   `Inode`/`FileOps` impl 面（`inode/mod.rs`）及其 VFS 转发路径**，而非
   合并上游 overlayfs diff。
2. **VFS 冲突面**：PR 触碰 `fs/vfs/fs_apis/inode.rs`、`fs_apis/xattr.rs`、
   `path/{mod,dentry}.rs`、`file/inode_handle.rs`；本仓 Wave5 曾改
   `fs_apis/{registry,inode,inode_ext}.rs`（root slot/task_ctx/inuse
   slot）——`fs_apis/inode.rs` 双方都改，需要真实 rebase 检验冲突规模。
3. **适配面盘点（待做）**：新签名涉及的方法清单要从 PR diff 精确提取
   （resize/set_mode/…/xattr 全族？缺省参数怎么给——部分 fs 无 Dentry
   可用，PR 里 ramfs atime 的处理是先例），再对照本仓
   `inode/mod.rs` 的 trait impl 与 `*_impl` helper 分散面。**这一步是
   本轮的第一个实质动作，待 user 指令后拉 PR diff 做逐方法对照表。**
4. **时点变数**：`mergeable_state: blocked` + lrh2000 的 `InodePublic`
   讨论未定稿——PR 自身还在演化，pick 早了可能要二次适配。

## 4. Open questions（user 裁决项）

1. **时点**：等 PR 落 main 后 rebase 适配，还是现在就 pick 冻结的版本？
2. **适配模型**：照 §18 定纲（Designer 冻结适配面 → Creator →
   Reviewer），适配Designer 输入 = PR diff 逐方法对照表（需先授权拉取
   PR files/diff）。
3. **VFS 写集**：本仓 VFS 文件（fs_apis/path 层）在本轮允许触碰的边界
   ——wave7 "尽量不动 VFS" 的裁决是否延续到本适配轮。

## 5. Next-main-agent actions

1. 待 user 裁决 §4 后：`gh api repos/asterinas/asterinas/pulls/3767/files`
   拉取逐文件 diff，产出「新签名 ↔ 本仓 impl 位点」对照表。
2. 对照表 → bounded Designer 冻结适配面 → Creator → Reviewer。
3. 动工前自检：无 ktest 面；除授权 VFS 写集外改动限于 overlayfs 目录。

## 6. Prohibitions（直至 user 指令）

不派发任何 packet；不改任何生产代码；不运行任何测试/编译。

## 7. 2026-09-04 追记：分支 fetch 与 pick 冲突模拟（只读，user 指令）

- **Fetch（未切分支/未建本地分支，仅 FETCH_HEAD）**：
  - PR head：`852f5644e`（fetch `asterinas/asterinas pull/3767/head`）。
    PR = 2 commits：`124e4f22b` "Pass Dentry to some Inode trait methods"
    + `852f5644e` "Adapt the new Inode trait interface to the actual FS
    implementation"。
  - 上游 main：`864b9138b` "Adjust the symlink implementation..."——
    **PR base = main 顶端**（无再演进）。
  - merge-base(ours, PR) = `604948581`；**我们落后 main 70 个提交**。
- **Pick 模拟**（`git merge-tree --write-tree --merge-base=<PR parent>
  HEAD <PR>`，纯内存，等价于把 PR 整体 pick 到本分支）：**15 个冲突**。
- **冲突分类（按"我们自 merge-base 是否触碰过该文件"机械判定）**：
  - **A 类·真冲突（我们改过 ∩ PR 改过，2 个）**：
    `fs/vfs/fs_apis/inode.rs`（我方 import/Wave5 时代改动 vs PR trait
    签名主战场）、`fs/vfs/path/dentry.rs`（我方 import 时代改动 vs PR
    dentry 变更）。
  - **A′·rename 伪冲突（1 个）**：
    `overlayfs/.agents/refactor/old/legacy_fs.rs`——我们的归档副本被
    rename 检测与上游 `overlayfs/fs.rs` 配对；处置 = 跳过（归档冻结，
    上游 overlayfs hunks 本就不可用）。
  - **B 类·上游漂移冲突（12 个，我们从未触碰）**：cgroupfs/inode.rs、
    devpts/mod.rs、devtmpfs/mod.rs（modify/delete：上游在我分叉后新建
    拆分文件）、exfat/inode.rs、ext2/impl_for_vfs/inode.rs、
    procfs/template/{dir,file,sym}.rs、ramfs/fs.rs、virtiofs/inode/
    mod.rs、systree_inode.rs、path/mod.rs——全部因落后 main 70 提交而
    冲突；先 rebase 到 main 即消失，pick 时按"取上游演进版 + 叠 PR
    hunk"机械解决。
- **关键结构性事实**：冲突清单里**没有 overlayfs 新实现的任何文件**——
  真正的适配工作不在 git 冲突里，而在 handoff §3.1 的"按新 trait 签名
  改本仓 overlayfs Inode impl 面"（PR 的 overlayfs hunks 对我们无效）。
- **策略含义（待 user 裁决时点）**：若先 rebase 到 main 再 pick，冲突缩
  至 A 类 2 文件 + overlayfs 手工适配；直接 pick 则 B 类 12 文件机械
  解决。"最终基于这个 PR"意味着 rebase 迟早发生——时点影响本轮成本。
- 本轮保持只读：未切分支、未建 ref、未动工作树（FETCH_HEAD 除外）。

## 8. 2026-09-04 追记：rebase 到上游 main 执行完毕（user 批准）

- **预处理**：证据冷备份 `/tmp/ovfs-agents-evidence-20260904.tar.gz`（850K，
  components+subagent-tasks）；`.vscode/settings.json` 入 stash（pre-rebase:
  local vscode prefs）；备份 ref `backup/pre-rebase-20260904` = `d7a2b93b7`。
- **Rebase**：`git rebase --onto 864b9138b 604948581`，29 提交线性重放，
  **仅 1 处冲突**——import 提交的 `overlayfs/fs.rs` modify/delete，按方针
  保留删除（legacy 归档在 .agents/refactor/old/；上游对旧 fs.rs 的语义
  演进待适配轮盘点）。**一次通过，分支现为 `a81a52858`。**
- **Pick 冲突面复测**：15 → **1**，唯一剩余 = legacy_fs.rs 归档
  rename 伪冲突（pick 时跳过）。A 类两文件（fs_apis/inode.rs、
  path/dentry.rs）被 rebase 消化。
- **编译门**：`cargo osdk check -p aster-core` **18 errors**——全部集中在
  overlayfs，全部源于 main 自身的 API 演进（非 PR #3767）：
  | 上游变化 | 证据 | 本仓受影响位点 |
  |---|---|---|
  | `sync_all`/`sync_data` 出 trait → 单一 `sync(mode: SyncMode)`（file_handle.rs:355） | E0407×2 + E0599×3 | inode/mod.rs:342,346（impl）、data.rs:110,114、copyup/mod.rs:450、inuse.rs:199?（调用） |
  | `unlink`/`rmdir` 增第 3 参 `child: &Arc<dyn Inode>`（fs_apis/inode.rs:451,455） | E0050×2 + E0061×5 | inode/mod.rs:415,423（impl）、copyup/mod.rs:212、inuse.rs:184,197（调用） |
  | `write_link` 出 trait → `create_symlink(name, target, mode)`（fs_apis/inode.rs:423，main "Adjust the symlink implementation"） | E0407 + E0599×2 | inode/mod.rs:427、dir/mod.rs:119、create.rs:61 |
  | `Path::new_fs_child` 删除 | E0599×3 | workdir.rs:91 等 |
  | `create_tmpfile` 增 `hard_linkability`（fs_apis/inode.rs:427） | E0061 | capabilities.rs:84,88,98 探针 |
- **定位**：18 错误全在 overlayfs 内（capabilities/inuse/copyup/workdir/
  data/dir/mod.rs/mod.rs），其中 create_symlink 替代 write_link 属**入口
  结构适配**（VFS 从"先 create 后写 target"改为单入口 create_symlink），
  其余为签名跟随。行为保持约束：unlink/rmdir 的 child 参数最小适配为
  接收不用（`_child`），内部 fresh-projection 语义不变——是否改用 child
  留给适配 Designer 裁决。
- **Next（待 user 指令）**：派 bounded Designer 冻结 18 错误适配面
  （输入 = 本表 + 现行 main API），→ Creator（compile preflight）→
  完成后复跑 make check + rustdoc → 再进入 PR pick（届时唯一冲突 =
  归档伪冲突）。rebase 后 make check/rustdoc 未跑（编译未过，先适配）。

## 9. 2026-09-04 追记：PR #3767 pick 完成（user 批准，含 stash pop）

- **Pick**：`git stash pop`（settings.json 回工作树，PR 不触碰该文件）
  → `git cherry-pick 124e4f22b 852f5644e`。冲突与模拟完全一致：仅
  `overlayfs/.agents/refactor/old/legacy_fs.rs` rename 伪冲突一处，按预案
  取 ours（归档冻结）。**分支现含两个 pick 提交（ccanging 署名保留）**：
  `7a88e88fa`（trait 签名）+ `ef5d603d2`（各 FS 适配，22 文件）。
- **编译全景（最终形态）**：`cargo osdk check -p aster-core` = **57 errors**
  （pick 前 18 → pick 后 57，全部 overlayfs）。完整日志归档
  `components/pr3767-merge-20260904/post_pick_check.log`。
- **最终 trait 形态（适配目标，fs_apis/inode.rs 实测）**——比"加 &Dentry"
  深：dentry 中心化重排，含签名塌缩：
  - 元数据全族：`set_*(self_dentry: &Dentry, …)`；
  - `link(self_dentry, old_dentry, name)` 双 dentry；
  - **`unlink(child_dentry: &Dentry)` / `rmdir(child_dentry: &Dentry)`：
    name 与 child 参数消失，dentry 一并承载**（塌缩）；
  - `rename(…)`（多 dentry）、`create/create_symlink/mknod`（多行签名）；
  - `get_xattr(name, writer)`/`list_xattr(ns, writer)` 无 dentry；
    `remove_xattr(self_dentry, name)`；
  - `sync(mode: SyncMode)`（main 漂移，与 PR 正交）；
  - `fallocate(mode, offset, len)` 保持无 dentry。
- **定位**：适配已不是签名跟随，而是 **overlayfs Inode impl 面的入口
  重设计**——需定义"如何从 child_dentry/self_dentry 映射到本仓
  (parent, name) fresh-projection 模型"（dentry.name/parent 提取点、
  dentry 与投影缓存的一致性、未用 dentry 的处置）。归 bounded Designer
  适配轮（输入 = 本节 + post_pick_check.log + 现行 fs_apis/inode.rs）。
- **Next（待 user 指令）**：派 bounded Designer 冻结适配面 → Creator →
  make check/rustdoc → merge 问题轮收尾。PR-handoff 文件与本轮记账尚未
  commit（随适配轮或 user 指令一并入库）。

## 10. 2026-09-04 收工记录（session close）

- **今日盘面**：分支 = main `864b9138b` + 29 重放提交 + PR #3767 两个
  pick 提交；编译 57 errors 待适配；备份齐备（`backup/pre-rebase-20260904`
  ref + /tmp 证据 tar；stash 已 pop 清空）。
- **适配设计状态**：**两 variant 均已冻结、均可直接执行**——Variant A
  （spec §2-§12：recorded_parent 降级保留 + 坐标改由操作 dentry 提供）、
  Variant B（spec §13：全删 recorded_parent + anchor-path 机制）。若上游
  采纳 rename 传新父 dentry 的修订，B-3 消失、方法表仅 rename 一行受影
  响，Creator 可从现有 spec 直接执行简化版适配；A→B 的选择因此**挂起等
  上游反馈**，不阻塞设计。
- **User 行动项**：向上游反馈 rename 新父 dentry 坍缩问题（VFS 分发层
  dentry.rs:765-767 持有 new_dir dentry 却在 :828-834 坍缩为 inode；
 Designer 建议 = trait rename 增/改传新父 dentry——即 follow-up 项）。
- **Next session**：① 依上游反馈裁 A/B → 派 Creator 执行适配（compile
  preflight + 回归套件 + make check/rustdoc + xfstests 全表 runtime
  gate）；② 适配落地后移除本 WIP 标记；③ 排队项照旧。
- **本 commit 性质**：WIP——分支处于「pick 完毕、适配未执行」的中间态，
  树不编译属预期；本提交仅含本 handoff 记录。

## 11. 2026-09-04 追记：Linux rename replace 契约调研（user 指令，主代理直查 /home/ayd/linux）

- **上游契约**：fs 端签名 = `(idmap, old_dir inode, old_dentry, new_dir
  inode, new_dentry, flags)`（include/linux/fs.h:2015）。**replace 不是
  接口元素**——它是目标 dentry 的正负状态：`vfs_rename` 里
  `target = new_dentry->d_inode`（namei.c:5952），负 → may_create、正 →
  may_delete；fs 负责 on-storage 替换（ext2 = `ext2_set_link` 把目标目录
  项改指向源 inode，namei.c 之后由 VFS 做 nlink/d_move/d_exchange）。
- **本仓对照**：PR 后 trait 把目标拆成 `new_dir_inode + new_name +
  replaced_dentry: Option<&Dentry>`，原因 = Asterinas 正 dentry 才有公开
  `Dentry` 句柄（negative 只是内部缓存表示，dentry.rs:1065-1087）。
  mode×Option 的可达组合实测仅三种：Create(absent)/Overwrite(positive)/
  Exchange(positive)——NoReplace+positive 在 VFS 层即 EEXIST，f 从不目睹；
  NoReplace+absent ≡ Replace+absent。**mode 与 Option 部分冗余。**
- **若 new_dir → &Dentry，replaced 的改形谱系**：
  (a) 最小：`new_dir_dentry` 替换 `new_dir_inode`，Option 原样；
  (b) 类型化塌缩（推荐提案形）：`dest: Destination<'_>` 枚举 =
  `Create{dir:&Dentry, name:&str} | Overwrite(&Dentry) | Exchange(&Dentry)`
  ——mode 与 Option 同时消失，三臂恰为可达组合；(c) Linux 镜像全塌缩：
  单个可负目标 dentry——需把负 dentry 升为一等公民，模型级改动。
  任一"dentry 新父"形态都解锁 B-3（新父命名空间位置经 name_and_parent
  可得），replaced 具体改形只动方法表 rename 一行。
- 依据 file:line：fs.h:2015、namei.c:5941-5952、ext2/namei.c:355-373
  （set_link replace）、dentry.rs:774-781（RenameMode）、dentry.rs:828-858
  （dispatch：`new_dentry.as_deref()` + 同/异 dir 两分支 + Exchange 双
  name_and_parent.set）。

## 12. 2026-09-04 追记：接口形态结论（user 问，主代理核证）

- **可达组合钉死**：`check_rename_mode`（dentry.rs:948-958）在 VFS 门
  NoReplace+positive → EEXIST、Exchange+negative → ENOENT——fs 的可达
  组合恰三种：Create(absent)/Overwrite(positive)/Exchange(positive)；
  mode 与 `Option` 部分冗余（NoReplace 的 EEXIST 语义整体属 VFS 门）。
- **推荐接口形态（不引入新 VFS struct）**：`enum RenameDestination<'a> =
  Create{dir:&Dentry, name:&str} | Overwrite(&Dentry) | Exchange(&Dentry)`，
  `rename(&self, old_child_dentry, dest, …)`——`new_dir_inode`→dentry、
  `new_name` 与 `Option` 与 `RenameMode` 三者同时退役；`replaced_xxx`
  命名问题消散（Linux 对该概念的词是 target，namei.c:5952）。最小档 =
  仅 `new_dir:&Dentry` + 参数改名 `target_dentry`。
- **用户 Linux 读法核证**：renamedata 只是参数束；目的地 lookup 物化
  可负子 dentry，`d_parent` 恒有效——**子 dentry 即自身与新父两个位置
  的承载者**，这是 Linux parents-as-inodes 无代价的原因，也是
  `ovl_copy_up(new->d_parent)` 的依据。
- 适配影响：方法表 rename 一行改写；两 variant 主体不变；上游反馈附上
  RenameDestination 提案形。

- **User 裁决（同日）**：RenameDestination 枚举否决（不引入新接口类型）。
  最小形态冻结为参数级改动：`rename(&self, old_child_dentry: &Dentry,
  new_dir_dentry: &Dentry, new_name: &str, target_dentry:
  Option<&Dentry>, mode: RenameMode)`——仅 `new_dir` 换 dentry + 参数
  更名 `replaced_dentry`→`target_dentry`（Linux 词汇，中性于
  Create/Overwrite/Exchange 三读法）；`new_name` 必须保留（None 时无
  dentry 承载名字，这正是 Linux 能省它的原因——它有负 dentry）。可达
  组合不变式（NoReplace+Some 不到 fs、Exchange 必 Some）冻结进参数 doc。
  B-3 解锁只依赖 `new_dir_dentry` 一处。

- **target 参数存在理由（主代理结论，完成 §11/§12 分析）**：位置确实被
  (new_dir_dentry, new_name) 隐含（user 判断成立——target 的位置字段纯
  冗余，ext2 甚至自己按名重查 `ext2_find_entry(new_dir,
  &new_dentry->d_name)`）；其不可替代的贡献只有两个：(1) **VFS 授权锚
  点**——may_delete/sticky-bit 预检跑在该对象上（dentry.rs:824-827），
  传参 = "授权替换的是这一个对象"，Linux 靠父 i_rwsem 全程覆盖使该锚
  无竞态；(2) 正负状态载体（Asterinas 无负 dentry 句柄）。去掉 = 转
  fs-authoritative 授权模型，是 VFS 安全语义变更非省参数。故 target
  保留、dentry 形态 PR 已改对、仅命名 → target；上游 ask 收窄为
  "new_dir 传 dentry" 一条。本仓 overlayfs 对该参数零消费（fresh
  projection），create.rs 甚至对 fresh-positive 目标报 ESTALE。

## 13. 2026-09-04 追记：replace target 全生命周期调研（user 问，主代理核证）

- **本仓 VFS 取参**：dispatch 在 `children.write()` 锁下
  `resolve_child_for_rename(new_name)`——缓存命中→`revalidate_cached_entry`
  （fs 级重验证）；未命中→**`inode.lookup(name)` 穿透现查**（dentry.rs:513）；
  ENOENT→None。故参数 = "dispatch 时经重验证或新鲜 lookup 的目的地正
  dentry 或 None"。后置门：check_rename_mode、跨目录 check_rename_cycle、
  sticky-bit 对 replaced_inode 预检。
- **Linux 被替换 target 四段生命周期**（namei.c:6064-6090）：① fs op 前
  VFS 做 may_delete/权限/锁 target；② **fs op 中由 fs 杀目标**——按名重查
  dirent + `inode_dec_link_count(target)`（ext2/namei.c 多处实证，**nlink
  是 fs 的活**）；③ fs op 后 VFS 接管 dcache/挂载层——目录目标
  `shrink_dcache_parent` + `S_DEAD`、一律 `dont_mount`+`detach_mounts`、
  `d_move`/`d_exchange`（FS_RENAME_DOES_D_MOVE 除外）；④ fsnotify_move
  收尾（target 作为被杀者入事件）。
- **本仓对照**：dispatch post-op 的 children.delete/insert +
  name_and_parent.set 即我们的 d_move；被替换 Dentry 直接被 insert 覆盖
  丢弃——无需 S_DEAD/shrink（我们的 dentry 非持久缓存对象，后续 lookup
  全部重新投影）；nlink/存储死亡由 recipe（remove/whiteout 机制）承担，
  与 Linux 的 fs-减-nlink 分工同构。
- **其它 fs 用法分类**：经典盘 fs（ext2/minix）= new_inode 做前置检查
  （目录 ENOTEMPTY/empty-dir）+ 按名重查 dirent + 自减 nlink；ext4 =
  同 + RENAME_WHITEOUT 自建 whiteout；内存 fs（shmem）= 自有树内搬移；
  网络/栈式（9p/ceph/ovl）= 名字转发或重推导真实表示；procfs/sysfs 类
  不实现 rename（默认 EPERM/ENOTDIR）。

## 14. 2026-09-04 终裁：Variant B 定案（user 上游反馈结果）

- **上游反馈结果**：new_dir 用 dentry "确实是更合理的形态"（上游认可），
  但**本仓可先自行修改**——本开发分支为 PoC 分支，最终 PR 将开干净分支
  做，故 follow-up（rename 传新父 dentry）由本仓在 PoC 分支自行落地。
- **决策**：**采用 Variant B**（spec §13.5 冻结面）：`recorded_parent`
  全删（含 `resolve_parent_object_id`、rename 重指、整个
  `ProjectionBinding` 枚举、`Arc::new_cyclic`、锁域 `RwMutex<Weak>` 全
  消失）；`..` 降级 = F1 自身 id 回退 + F5 anchor 精确解析（机制 §13.4
  已冻结）；**B-3 缺口随本仓自行落地的 rename dentry 化一并消失**（新父
  命名空间位置 dentry 化）。Variant A（§2-§12）降为设计存档，重叠处以
  §13 为准。
- **执行面（下一 Creator pass，待派发）**：
  1. VFS 侧三处：trait `rename` 签名（fs_apis/inode.rs：
     `new_dir_inode` → `new_dir_dentry: &Dentry`）、dispatch 转发
     （dentry.rs：`DirDentry::rename` 已持有 `new_dir: &DirDentry`，
     直接传 dentry 不再坍缩）、`Dentry::parent` 可见性
     `pub(super)`→`pub(in crate::fs)`（spec §8）。
  2. 其余实现者机械适配（7 个：ext2/exfat/ramfs/procfs/virtiofs/
     devpts/cgroupfs——`.inode()` 转发一行为主）。
  3. overlayfs 主体：spec §13 Variant B + 方法表（rename 行按 §12 最小
     形态改写：`new_dir_dentry: &Dentry` + `target_dentry: Option<&Dentry>`
     命名）。
- **验证义务**（designer_validation §7 Variant B 激活项）：compile →
  回归套件 → make check + rustdoc → **xfstests 全表且必须组合
  "祖先改名后 rename 进 lower 子目录"序列（VB-1）**；ktest 可选。
- 状态：本 handoff 与 WIP commit 同步 amend；Creator 派发待 user 指令。
