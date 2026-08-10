<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: 2026-08-05 Wave7 xfstests Sequencing

**Date / Time:** 2026-08-05
**Status:** `Wave7 运行中（2026-08-08 综合用例批次完成）。本轮 6 例：
029/077/038 PASS（038 经 pass_40 impure 修复后验证通过）；031 范围内
VERIFIED（两次 ENOTEMPTY 已修复，仅剩 ls3 = Bug B 依赖失败，已登记）；
020 NOTRUN（userns 能力门）；041 NOTRUN（xino=on 挂载选项回显缺口，已
登记，属 procfs/VFS 接口决策）。Bug B（base 视图一致性）与 041 回显均为
独立待办，触碰 VFS 边界需用户决策；跨用例 dirent 残留同属 Bug B 族，
修复前每例重建镜像。078 与 stress 用例（001/021/019）按用户指示排除。
生产 `.rs`（pass_40，9 文件）未提交待用户指示。`

**2026-08-10 状态更新（修正，取代上段中 031/Bug B 的旧表述）：**
- **031 已整例 PASS（2026-08-09 单跑 `Passed all 1 tests`）**——Bug B 的
  ls3 缺口在 pass_42 + VFS 可见性放宽（commit `e3da18bd9`）后已修复；
  任何"031 仅剩 ls3 / Bug B 阻塞"的表述均过时。**Wave7 已整例通过 20 例**
  （含候选增补 022/063），完整清单见 §8.5 / §8 统一账。
- **pass_43_wave7_cache_consistency 已完成并 compile-accepted**（commit
  `7d6b5c960`；三阶段 Creator + 编译门 0 错误，仅预存 `uuid_mode` 警告）。
- **overlay/021 已定案（2026-08-10 冒烟）**：FAIL 根因 = **Asterinas 内核缺
  aio 能力**（`io_setup` syscall 206 → ENOSYS）→ fsstress 在任何文件操作前
  退出 → 021 种植 0 文件 → glob 全空；**非 overlayfs 缺陷**，无 overlayfs
  修复动作（记录见 §8 统一账 021 行 / NEXT ACTION）。
- **pass_43 meso-integration Checker 已验收（OUTCOME A，2026-08-10）**：
  12 例可调度回归批次 `002/003/006/007/010/011/012/014/024/031/038/077`
  初跑 11 PASS / 1 FAIL（012，Change 1 stale-upper 回归）→ pass_44 修复
  （`cd29d9c17`）→ 复验 **12/12 PASS、0 NOTRUN、0 HANG**。pass_43 +
  pass_44 **gate-accepted**；§8 统一账 012 恢复绿色（20/43）。


**2026-08-09 更新（Bug B Path 设计派发 + 结构性验收）：**
`task_designer_wave7_bug_b_path_20260809` 经 V2 派发 lane 执行（用户贴派发轮 →
spawn `fork_turns="1"`），产物结构性 **ACCEPTED**：
`components/wave7_bug_b_path_design/wave7_bug_b_path_designer_spec.md` +
`_designer_validation.md`。冻结面：layer-root `Path` 锚点
（`OverlayLayer.root_path`）、`RealObject.real_path: Option<Path>`（唯一 `None`
= readdir `..` 身份投影）、`WorkdirTemp`/`WhiteoutHandle` 改 dentry 锚定 `Path`、
`upper_parent_path()`/`workdir_root_path()` owner-private 包装 + 全部写点直接
`Path` 方法调用；unmount 双向处置（overlay 先卸 = last-Drop RAII；base 先卸 =
「实际无所谓就不管」，`Mount::do_unmount` 无引用门、持有引用对卸载与 `Path`
有效性无差别，不设计失效机制）；确认零 VFS 接口改动、无新锁域、无 ktest。
下一步：Creator pass 切片已记录（`PASS_SLICING.md` `pass_42_wave7_bug_b_path_repair`，
一 pass 三阶段：A 载体/锚点/查找路由、B workdir temp + copy-up/link 腿、C dir 系
语义扫尾；中途不编译，Phase C 后主代理只修机械编译错误，每阶段 git diff 单阶段
呈现、B/C amend Phase A commit）；Phase A 已验收并提交（`f68ecdcec`，
codex/overlayfs-refactor，5 文件 +182/-38）；Phase B 已验收并 amend 进同一
commit（`c5d7e36db`，11 文件 +369/-143）；Phase C 已执行并 amend 进同一
commit（`4830cd007`，12 文件 +582/-314），但首次编译失败：8 处
`Dentry::lookup_child` 不可达（方法实际在 `DirDentry` 上、构造器
`pub(super)`）——Designer 修订（改用 pub 的 `PathResolver::lookup_at_path`）
已派发并返回**阻断性升级**（同线程 resolver 写锁自死锁），见 §4。

**2026-08-10 更新（Cache 一致性设计派发 + 结构性验收）：**
`task_designer_wave7_cache_consistency_20260810` 经普通 spawn（fork_context=false
平台试验；首个 fork 模式子代理误读父上下文已关闭）完成，产物结构性
**ACCEPTED**：`components/wave7_cache_consistency_design/wave7_cache_consistency_designer_spec.md`
+ `_designer_validation.md`。四项代码设计 + 一项文档边界：Change 1 BindingCache
验证式 memo（lookup_binding 每次层扫描 + `Binding::matches_truth` 身份一致才
serve）；Change 2 `InodeCache::get_or_create` 同对象校验 + 陈旧条目替换
（F1）；Change 3 `alias_key`/`replace_facts` 位移分支细化（同对象=F2 保留
Err；异对象=ino 复用陈旧占用替换自愈）；Change 4 `BindingCache::invalidate_parent`
+ 父 copy-up 旧 key 清理（F3）；Change 5 readdir 边界 A（文档）。新函数全部为
最优持有者方法；(a)(b)(c) 三表逐项 file:line disposition（lookup_binding 8 个
调用方核实）；零 VFS 改动、无新锁域、无 ktest；micro 9 个 ID 已核对。下一步：
Creator pass 切片（待用户指示）。

**2026-08-10 更新（派发通道升级：Direct Spawn Lane 优先，V2 备选）：**
普通 spawn（`fork_context=false` + 自包含初始消息）经 3 次验证可重复：Designer
任务（Planck，精确 write-set 完成）+ pingpong round1（Carson）/ round2（Russell）
均正确收到消息并回复；fork 模式（Galileo）误读父上下文、违反 write-set 已弃用。
已修订 `PROTOCOL.md` §1.3 与 Core Terms：**Direct Spawn Lane 为优先项**（无需
用户贴派发轮；初始消息携带角色声明/task_id/规则路径/packet 路径/write-set/
禁止项/报告契约），**V2 User Dispatch Turn lane 降为备选**（严格零载荷指针
路由时使用）。`.agents/skills/ovfs-main/SKILL.md` 因只读未改（skill 声明以
PROTOCOL.md 为准）。

**2026-08-10 更新（061 归因修正，主代理亲自全量打点）：** `overlay/061`
经 3 轮单例打点运行（全量 overlay 探针 + VM 缺页/回写/munmap 探针）定位到
**通用 VM/page-cache 缺陷**：MAP_SHARED mmap 写不标记 CachePage dirty →
unmount 回写丢失 → after-cycle 读旧数据。2026-08-09 的"双 copy-up/同 key
双载体"归因未复现；overlayfs 投影/InodeCache 侧本场景无缺陷（单载体、单
promote、alias 15→17 OK）。详见 §8.4 与
`components/wave7-xfstests-sequencing/overlay061_reinvestigation_20260810.md`。

## 1. Global State Pointer

- **Current Active Wave / Pass:** Wave7 xfstests sequencing
  (`wave7_xfstests_sequencing_20260805`), started (case 1 run 1 complete;
  table cleaned and re-sorted 2026-08-07).
- **Predecessor:** Wave6 documentation review is closed. Its static chain
  (workspace Clippy, `cargo fmt --check`, and `make check`) passed; this does
  not constitute runtime xfstests evidence or final acceptance.
- **Blueprint Updates Made:** No. This handoff is the user-requested planning
  record only; `SYSTEM_BLUEPRINT.md` and `PASS_SLICING.md` retain their
  accepted Wave6 state until Wave7 is expressly started.
- **Scope source:** Stage D makes all P0/P1 and `P2-01 xino` mandatory, and
  the later accepted scope amendment also includes `P2-11 UUID modes`.
  The six accepted Designer validation contracts remain the external-evidence
  source. Source reconciliation (2026-08-07) shows no packaged case observes
  `P2-11` (Section 3); xfstests is many-to-many evidence, not a
  one-test-per-micro claim.

## 2. Ordered Current-Scope xfstests Obligation

Run the following cases one at a time, in this order. Before attributing any
result, the Checker must confirm the actual case setup and asserted theme from
the upstream suite source, preserve the result file and guest log, and record
`PASS`, `FAIL`, or `NOTRUN` per case. A failing case follows the normal Checker
evidence and repair-loop protocol; this table does not authorize a repair or
rerun.

**2026-08-07 reconciliation:** every "Current-scope purpose" below was
re-derived from the upstream suite source (`tests/overlay/<case>`), replacing
the previous mis-attributed themes. Sort key = functional foundation (mount /
read-only fundamentals first; then copy-up and file views; then whiteout and
namespace semantics; then xattr/metadata; then permission/credential/stacked
behavior; then cache invalidation and xino) and, within a tier, test-unit
simplicity (`S1` = scratch + mount only; `S2` = one extra standard dependency;
`S3` = user/group/unshare/relatime or multi-scenario setup; `H` = heavy data
or concurrency). Cases needing the deferred `index=on` feature or a loop/XFS
backing lane are listed after the main table and are not schedulable now.

### 2.1 Schedulable cases (32), in execution order

| Order | Case | Upstream theme (from suite source) | Foundation tier | Simplicity |
| ---: | :--- | :--- | :--- | :--- |
| 1 | `overlay/035` | Read-only mount cases: no `upperdir`; immutable workdir forces read-only mount; no remount to rw. | Mount / read-only fundamentals | S2 (`chattr i`) |
| 2 | `overlay/024` | `workdir/work` leftovers must be cleaned at mount; mount must stay rw. | Mount / read-only fundamentals | S1 |
| 3 | `overlay/009` | `default_permissions` mount must not leak dentries (mount + read + unmount). | Mount / option behavior | S1 |
| 4 | `overlay/002` | fsync on a merged-directory file (write through overlay then fsync) must not crash. | Basic file I/O | S1 |
| 5 | `overlay/007` | getcwd must not fail after an unsuccessful rmdir on the test dir. | Basic path/name resolution | S1 |
| 6 | `overlay/016` | ro/rw fd consistency: read fd must not see stale data after write-through copy-up. | Copy-up + file views | S1 |
| 7 | `overlay/013` | truncate of running executables from lower/upper must fail `ETXTBSY`. | Copy-up + file views | S2 (helper `t_truncate_self`) |
| 8 | `overlay/004` | Copy-up triggered by mode-bit change; unprivileged user cannot trigger it. | Copy-up + file views | S2 (`_require_user`) |
| 9 | `overlay/003` | Basic whiteout visibility sanity when upper fs lacks `d_type` (mount gate). | Whiteout / visibility | S1 |
| 10 | `overlay/010` | rmdir of a lower dir containing whiteouts must not crash. | Whiteout / visibility | S1 |
| 11 | `overlay/012` | Stale upper dentry (removed from upper behind the overlay) must not warn/oops on remove. | Whiteout / stale state | S1 |
| 12 | `overlay/006` | Rename from lower to upper must not leave a visible whiteout. | Whiteout / visibility | S1 |
| 13 | `overlay/031` | Invalid whiteouts after lowerdir change + remount must not be exposed; rmdir must work. | Whiteout / remount | S2 (multi-mount) |
| 14 | `overlay/008` | Create over a whiteout as another user: uid/gid must be the creator's, not the mounter's. | Whiteout / permissions | S2 (`_require_user`) |
| 15 | `overlay/015` | SGID bit inheritance over a whiteout. | Whiteout / permissions | S2 (`_require_user` + `_require_group`) |
| 16 | `overlay/011` | `trusted.overlay.opaque` on an intermediate lower must be hidden from users. | Xattr / metadata | S2 (`trusted` attrs) |
| 17 | `overlay/026` | Xattr prefix filter: `trusted.overlay.*` rejected, `trusted.overlayxxx` allowed. | Xattr / metadata | S1 |
| 18 | `overlay/023` | Workdir must not inherit default POSIX ACLs from its parent. | Xattr / metadata | S2 (`_require_acls`) |
| 19 | `overlay/014` | Copy-up of an opaque-xattr lower dir must not copy the opaque flag. | Xattr / copy-up | S1 |
| 20 | `overlay/020` | Copy-up/namespace/cred: mounter superblock creds must be used (uses `unshare`). | Credential / namespace | S3 (userns) |
| 21 | `overlay/025` | POSIX ACLs on tmpfs-backed lower/upper layers must be readable. | ACL / layered backend | S3 (`tmpfs` extra fs + user) |
| 22 | `overlay/029` | Stacked overlay (overlay mounted as lower of another overlay) must resolve via `d_real`. | Stacked layers | S2 (nested mount) |
| 23 | `overlay/040` | Writing ioctl on a lower file must return `EPERM`, not modify the lower file. | Metadata / fileattr | S2 (`chattr i`) |
| 24 | `overlay/027` | Immutable upper file must stay untouchable through the overlay. | Metadata / fileattr | S2 (`chattr i`) |
| 25 | `overlay/078` | Copy-up of lower file attributes (immutable/append/sync/noatime) across copy-up + mount cycle. | Metadata / fileattr | S3 (`chattr ASai`, `syncfs`, scratch shutdown) |
| 26 | `overlay/039` | relatime updates for directories in the upper layer. | Metadata / time | S2 (`relatime`) |
| 27 | `overlay/077` | Readdir cache invalidation on changes to a dir with origin; stale entries skipped. | Readdir cache | S1 |
| 28 | `overlay/038` | Consistent `d_ino` for same-filesystem setup. | xino (`P2-01`) | S2 (`t_dir_type` + `trusted` attrs) |
| 29 | `overlay/041` | Consistent `d_ino` for non-same-filesystem setup (variant of 038). | xino (`P2-01`) | S3 (non-samefs setup + `t_dir_type`) |
| 30 | `overlay/001` | Copy-up of lower files <, =, > 4 GiB (`O_LARGEFILE` regression); needs ≥8 GiB free on scratch. | Copy-up (large file) | H (≥8 GiB scratch) |
| 31 | `overlay/021` | Concurrent copy-up of many files (copy-up bombs, 4 parallel teams); needs ~16 GiB scratch. | Copy-up (concurrency) | H (≥16 GiB scratch) |
| 32 | `overlay/019` | fsstress concurrently on lower and top dirs. | Stress | H (stress) |

### 2.2 Not schedulable in the current scope (6)

| Case | Upstream theme (from suite source) | Why not schedulable now |
| :--- | :--- | :--- |
| `overlay/005` | Copy-up error path memleak → panic on unmount; uses loop devices + XFS backing. | `_require_loop`; harness lane is ext2 images, no loop/XFS backing lane; scope decision needed. |
| `overlay/032` | Concurrent copy-up of lower hardlinks. | `_require_scratch_feature index`; `index=on` is deferred P3 behavior. |
| `overlay/033` | nlink accounting for overlay hardlinks. | `_require_scratch_feature index`; deferred. |
| `overlay/034` | nlink with unaccounted lower hardlinks. | `_require_scratch_feature index`; deferred. |
| `overlay/037` | Mount error cases with `index=on` (ESTALE on mismatch). | `_require_scratch_feature index`; deferred. |
| `overlay/042` | Lookup of non-indexed upper after offline lower-hardlink creation. | `_require_scratch_feature index`; deferred. |

The order is intentional: mount and read-only fundamentals precede copy-up and
file views; whiteout and namespace semantics precede xattr/metadata and
credential/stacked behavior; cache invalidation follows the readdir/xino
baselines it depends on; heavy data and stress cases are last because they
need large scratch images and long runtime.

## 3. Explicit Scope Exclusions

- Do not schedule `overlay/017`, `043`, or `057`: they require deferred
  `P2-02 redirect_dir` behavior, even where they incidentally observe a basic
  stat or xino path.
- Do not schedule `overlay/018`, `028`, or `044` as required whole-case
  passes: their lower-hardlink/origin/nlink assertions need deferred
  association/index or `P2-07` behavior. The current basic contract permits
  only the non-index upper-authoritative link observation, not treating a
  whole mixed-scope case as passed.
- Do not schedule `overlay/030`, `075`, or `076` (deferred `P2-06`
  fileattr), `overlay/083`, `084`, or `109` (deferred userxattr/escaping or
  unavailable packaged lane), or any P3 index, NFS export, metacopy,
  data-only, fs-verity, trap, nested-overlay, or cleanup case.
- Do not schedule `overlay/005` (requires loop devices and an XFS backing
  lane not present in the packaged ext2-image harness), nor `overlay/032`,
  `033`, `034`, `037`, or `042` (all require `_require_scratch_feature index`,
  i.e. deferred `index=on` behavior). See Section 2.2.
- `overlay/029` stays schedulable in Section 2.1: its stacked-overlay setup
  exercises the P0/P1 `d_real` path with only `_require_scratch`, which the
  prior generic "nested-overlay" exclusion was not aimed at; the P3 nested
  overlay/cleanup class remains excluded.
- Do not schedule `overlay/067` or `068` as UUID tests, and do not treat
  `overlay/078` as a UUID observation: source reconciliation shows 078 is a
  fileattr copy-up test (`copyup perms shutdown`) requiring `chattr ASai`,
  `syncfs`, and scratch shutdown, and it now sits with the deferred `P2-06`
  fileattr class. The staged inventory's dedicated `overlay/081` UUID row is
  absent from the packaged `full.list`. Until an explicitly authorized
  source-level reconciliation changes that mapping, no packaged case is a
  current-scope UUID observation.

## 4. Thread Activity Log

- **V2 子代理派发机制实测与协议修订（2026-08-08，user-directed；机制验证
  探针 + 端到端冒烟均通过）：** 只读探针（probe4/5/7 可见性汇报、probe7
  对照、probe6 主代理错乱样本）实测确立以下平台机制事实：
  - `fork_turns="1"` 时子代理模型输入 = 系统提示（含 skill 目录与全部工具）
    + **最新一条 user 轮**；spawn payload、NEW_TASK 头、followup/send_message
    内容均不可读；助手消息（含 spawn 前同轮内容）不进入模型输入；
    **UI/线程上下文 ≠ 模型上下文**（UI 显示父会话内容、模型读不到——
    ASSISTANT-MARKER-V7 探针确认"存在但不可见"）。
  - 子代理 `get_goal` 恒为 `null`（根线程 active 时亦然）；goal 注入不是可
    fork 的 user 轮；**goal 常驻契约无法自动到达子代理**，goal 模式只能
    自动化主代理侧。
  - 身份由最新 user 轮措辞锚定 + `list_agents` 交叉验证；若最新 user 轮是
    面向主代理的消息，子代理会主代理错乱（probe2/3/6，probe3 曾递归 spawn
    子代理，均已中断）。
  据此协议修订（user-confirmed）：`PROTOCOL.md` §1.3 重写为 User Dispatch
  Turn 派发契约（`fork_turns="1"` 唯一允许、`all` 禁止、`none` 不可用；
  续派 = 新派发轮 + Continuation 指针；验收只看子代理产物）；新增
  `protocol/templates/user_dispatch_turn_TEMPLATE.md`；`$ovfs-main` /
  `$ovfs-subagent` / `$ovfs-checker` 三个 skill 同步（V2 派发五步 lane、
  Delivery facts、运行时授权仅来自 packet）。
  端到端冒烟（`task_smoke_v2_dispatch_20260808`；packet 见
  `subagent-tasks/v2_dispatch_smoke/`）通过：子代理从磁盘读 packet、
  `list_agents` 确认身份、按 write-set 产出 receipt 至
  `components/v2_dispatch_smoke/smoke_receipt_20260808.md`，未越权、
  未跑命令。后续所有子代理派发一律走此 lane。

- **综合用例批次（2026-08-08，user-directed；main agent 亲自执行，无
  subagent；每例单独 runlist、单独 QEMU，按需重建镜像）：** 结果矩阵与逐例
  证据见
  `components/wave7-xfstests-sequencing/pass_39_wave7_comprehensive_cases_checker.md`；
  日志按用例归档于 `run_evidence/<case>/comprehensive_20260808/`；上游源码
  （020/031/038/041）归档于 `run_evidence/upstream_sources/20260808/`。
  - PASS（2）：`029`（嵌套 overlay d_real）、`077`（readdir 缓存失效）。
  - FAIL（2）：
    - `031`：lowerdir 变更后重挂载，上层遗留 whiteout 成为非法 whiteout；
      期望 `Silence is golden`（ls 可见 + rm -rf 成功），实际
      `rm: cannot remove 'SCRATCH_MNT/testdir': Directory not empty`（×2）
      且 `ls: cannot access '.../ovl-mnt/testdir': No such file or
      directory`。缺口：合并目录内 whiteout 残留的 remove 路径
      （meso 06 `dir/remove.rs`/`dir/whiteout.rs`）与重挂载可见性
      （meso 03/06 readdir/lookup）。
    - `038`：目录因 copy-up 文件迁入变 impure 时未持久化
      `trusted.overlay.impure=y`（`No such attribute`）。该用例其余
      d_ino/xino 与 readdir 缓存断言全部通过；即 031 之外唯一断言缺口是
      impure 标记写入/清除（meso 04 copy-up + meso 05 xattr）。
  - NOTRUN（2）：`020`（`unshare -m -p -U: not supported`）、`041`
    （`cannot enable xino feature`；`-o xino=on` 挂载选项未实现，P2-01 的
    选项面缺口——038 证明 xino 投影本身在 samefs 下工作）。
  - **跨用例污染发现（031 → 020 attempt A）：** harness
    `_scratch_mkfs`/`_scratch_cleanup_files` 的 `rm -rf
    $OVL_BASE_SCRATCH_MNT/*` 在复用镜像时失败：
    `rm: cannot remove '/opt/xfstests/scratch/lower1': Directory not empty`。
    debugfs 取证：盘上残留仅为空目录 `lower1/testdir`（仅有 . / ..），但
    其目录块第 28 字节起残留一个未合并的陈旧 dirent 记录（inode 4088、
    name `a`、file_type 1）——即 overlay 删除 whiteout 字符设备时 base
    ext2 的 dirent 未正确清除/合并，内核 readdir 仍视为活条目 → rmdir
    ENOTEMPTY。修复批次含该 base-fs dirent 缺陷（先诊断 overlayfs
    whiteout-unlink 路径还是 ext2 驱动，再路由）；在修复前每例必须重建
    镜像（后续 5 例均按此执行）。
  - **执行形态：** 每例前删除生成镜像 → `make run_kernel ...` 重建全新
    8 GiB ext2（新 UUID，逐例记录）；最终镜像哈希（041 运行后）TEST
    `415721c967937980516d9f452bf0e1c1029b50b175f3c30238b27908f3302841`、
    SCRATCH `6c2b9669f2d629d00e6dec8ac58b44f34c94269546c620f810b3094ef4a3a964`；
    批次后临时 runlist 与镜像均已删除。

- **Impure 标记修复登记（2026-08-08，user-directed；先登记，修复执行另待
  授权）：** `overlay/038` FAIL 的唯一断言缺口（
  `trusted.overlay.impure` 缺失）登记为独立修复目标。
  - 目标语义（Linux 契约）：上层目录内出现 whiteout、opaque 或带 origin
    的 copy-up 子项时，写入 `trusted.overlay.impure=y`（`ovl_set_impure`
    等价）；目录重新变 pure（最后一项杂质清除）后移除标记
    （`ovl_clear_empty`/清除路径等价）。
  - 代码现状（2026-08-08 核对）：读侧已分类——
    `metadata_security/xattr.rs` 的 `OVERLAY_PRIVATE_SUFFIXES` 含
    `"impure"`（对用户隐藏/拒绝）；写侧完全缺失——全树无写入点，owner
    分发表注释也未给 impure 指定 owner（whiteout/opaque→meso 06、
    origin/upper→meso 02、nlink→meso 04、uuid→meso 01、
    metacopy/protattr→deferred；impure 无条目）。
  - 触发路径（038 证据）：`mv` 将 copy-up 文件迁入纯上层目录 → 目录变
    impure（断言 `trusted.overlay.impure == y`）；随后清理文件与子目录 →
    目录变 pure（断言标记消失）。写入点候选：copy-up-into-dir 与
    whiteout 创建侧的上层父目录（meso 04 `copyup/`、meso 06
    `dir/whiteout.rs`）；xattr 名称/值常量按现有 `OPAQUE_XATTR_FULL_NAME`
    模式声明于 `metadata_security/xattr.rs`（meso 05）。
  - 验证映射：`overlay/038` `direct`（出现 + 清除两处断言；该用例其余
    d_ino/xino/readdir 断言当前已 PASS，修复后须保持）。
  - 边界：不动 VFS 接口；无 ktest；本记录只是目标登记，不构成 Creator
    切片——下一步按 `wave7_logic_bug_repair_20260807` 模式走 bounded
    Designer addendum 冻结精确 Rust 表面（impure 常量、set/clear 触发
    点、parent meso 归属），Creator 落地后由 main agent 做 compile
    验证，QEMU/xfstests 重跑需用户另行授权。
  - 与 031 的关系：031 的 whiteout 暴露/rmdir 缺口与 base-ext2 dirent
    残留缺陷各自单列，不并入本目标。

- **031 探针定位（2026-08-08，user-directed；临时 `println!` 探针
  `[ovfs031]`，无等级门控、已在 serial 验证可输出）：** 5 处探针
  （lookup_binding / lookup_in_layers / remove_target / readdir_sequence /
  publish_whiteout），完整轨迹与盘上取证见
  `run_evidence/overlay031/probe_20260808/`
  （`ovfs031_probe_trace.txt` + `ovfs031_probe_analysis.md`）。
  - **确证 A —— 两次 ENOTEMPTY（rm1/rm2）同一根因：** `dir/remove.rs`
    `remove_target` 的 **pure-upper rmdir 分支**——`visible_child_count`
    门返回 0（whiteout 不可见，正确），`is_pure_upper=true` 后直接
    `upper_parent.rmdir(name)`，而上层目录物理残留 whiteout 'a'（block A
    发布在自动创建的上层 testdir 内）→ 底层 rmdir `ENOTEMPTY`。pure-upper
    分支没有 Linux `ovl_cleanup_whiteouts` 等价步骤；clear-empty 机制只在
    lower-backed 分支。rm2（lower 同名被 touch 成文件）因上层目录仍遮蔽
    lower 而走同一路径。
  - **确证 B —— ls3 ENOENT（testdir 不可见）：** rm3（lower-backed rmdir，
    语义正确，publish err=None）发布 whiteout "testdir" 到上层根后，该
    whiteout（字符设备 mode 0000）在后续 base fs `rm -rf $upperdir/testdir`
    中未被移除（base 视图未看到 overlay 直写条目，与已登记
    workdir-cleanup VFS-parity 分歧及跨用例 dirent 残留同族）→ 后续挂载
    lookup 命中 whiteout → testdir 隐藏 → ls3 ENOENT 且 mount #6 的 rm
    静默失败。debugfs 佐证：`/upper/testdir` 为字符设备 mode 0000 存续。
  - 附加观察：`/lower1` 存在重复 `testdir` dirent（inode 26/28），同族
    陈旧 dirent 残留。
  - 处置：探针为行为中性 println!、编译通过，保留待 B 项 base-fs 视图的
    补充探针；未做任何修复（用户：先不修）。

- **impure + cleanup 设计调研派发（2026-08-08，user-directed；Designer
  经 WSL codex CLI 派出，非 subagent）：** `task_designer_wave7_impure_cleanup_20260808`
  同时研究两个方向的全部修改点——(1) impure 标记持久化（038：set/clear
  触发点、owner 归属、常量/签名、private 过滤交互）；(2) rmdir 前 whiteout
  清理（031：全分支盘点、修复形态决策、对现存 clear-empty 实现的显式处置）。
  Bug B（base 视图一致性）明确 OUT OF SCOPE，只记依赖边。派发包
  `subagent-tasks/wave7_impure_cleanup_design/task_designer_wave7_impure_cleanup_20260808_dispatch.md`；
  派出方式 = aster-code-review `run_agent.sh` + `codex` profile（私有
  CODEX_HOME、继承 auth、`codex exec`，gpt-5.5/high/workspace-write），
  CLI session `019fdf54-75fa-7c70-acd4-d149710ab2eb`。待产物：
  `components/wave7_impure_cleanup_design/` 两个设计文件；接受后按协议做
  结构性验收并登记 Creator 切片。

- **Designer CLI 派发结果（2026-08-08，环境故障后已恢复落盘）：** 首次派出
  （aster-code-review codex profile，钉死 gpt-5.5/openai + 私有 CODEX_HOME）
  与用户 OSS 配置不符、连 chatgpt.com 持续超时，已废弃。改用继承
  `~/.codex` 直接 `codex exec`（deepseek-v4-flash / custom provider /
  workspace-write）：Designer 完成全部调研并给出完整冻结设计（impure 的
  T1-T4 set 触发点 / C1-C2 clear 触发点 / `IMPURE_*` 常量 +
  `OverlayXattrPolicy::{has,set,clear}_impure_marker` 内部接缝；whiteout
  cleanup 的全分支盘点 / pure-upper rmdir `cleanup_upper_whiteouts` 预扫 /
  clear-empty 显式处置 / `is_whiteout_inode` 谓词抽取；031 ls3 与跨用例
  dirent 残留记为 Bug B 依赖）。运行中途平台 exec 机制故障导致首次无法
  写盘；桌面端重启 + WSL codex 更新（v0.147.0）后，已通过
  `codex exec resume 019fdf62-c50a-7eb0-9028-ccefee502861` 恢复同一会话
  落盘两个产物（`components/wave7_impure_cleanup_design/`）。探针
  （`[ovfs031]` println!）已按用户指示从 5 个代码文件清除（工作树恢复
  HEAD）。
- **Designer 调研验收（2026-08-08）：ACCEPTED 结构性验收通过。** 产物
  `components/wave7_impure_cleanup_design/wave7_impure_cleanup_designer_spec.md`
  （718 行）与 `_designer_validation.md`（136 行）已落盘；micro ID 已对账
  到清单（P2-03 / P1-33 / P1-01..07 / P1-25..28 / P1-31 / P1-36）。冻结面
  摘要：impure set 触发点 T1 promote（copy-up-into-dir）、T2
  publish_whiteout、T3 rename_upper、T4 link_impl；clear 点 C1
  remove_target、C2 rename_upper；内部接缝
  `OverlayXattrPolicy::{has,set,clear}_impure_marker` +
  `OverlayInode::refresh_impure_marker` + `IMPURE_*` 常量（沿 OPAQUE_* 模式）。
  cleanup：pure-upper rmdir 分支增加 `cleanup_upper_whiteouts` 预扫（Linux
  `ovl_cleanup_whiteouts` 等价，非 whiteout 残留拒绝 ENOTEMPTY）；现存
  clear-empty 保留给 lower-backed 分支、其 displaced-dir 清理腿并入同一扫
  描接缝且保持 best-effort；`is_whiteout_inode` 谓词从 entry.rs 抽取。
  Bug B（base 视图一致性）仅作 out-of-scope 依赖记录（031 ls3 与 031→020
  残留预期在 Bug B 修复前保持 FAIL，不视为本设计失败）。下一步：待用户
  授权后按 PASS_SLICING 记录做 Creator 切片。
- **Creator 派发（2026-08-08，user-directed；主线程验收）：**
  `pass_40_wave7_impure_cleanup` 经 WSL codex CLI 派出（继承 `~/.codex`，
  deepseek-v4-flash），实现 ACCEPTED 设计的两个目标（impure T1-T4/C1-C2 +
  `IMPURE_*`/`OverlayXattrPolicy::{has,set,clear}_impure_marker`/
  `refresh_impure_marker`；cleanup `cleanup_upper_whiteouts` 预扫 +
  `is_whiteout_inode` 抽取 + clear-empty 腿并入扫描接缝）。写集 8 个 `.rs`
  + 报告，command-free（compile 预检被收回）。验收流程：主线程 exact-diff
  对照冻结面 → 容器内 `cargo check -p aster-kernel --target
  x86_64-unknown-none` → 登记接受。Bug B 不入实现范围。
- **Creator 验收（2026-08-08）：ACCEPTED（含已记录偏差）。** 实现覆盖
  冻结面：impure（`IMPURE_*` 常量、`OverlayXattrPolicy::{has,set,clear}_impure_marker`、
  `OverlayInode::refresh_impure_marker`、T1-T4 严格 pre-commit 触发、
  C1/C2 best-effort 刷新、owner 注释更新）；cleanup（`is_whiteout_inode`
  抽取 + 委派、`cleanup_upper_whiteouts` 扫描接缝、branch-A pure-upper
  rmdir 预扫、branch-E rename 预扫、clear-empty 腿并入扫描接缝）。偏差 6
  项全部记录（第三个 `ReaddirIndex::entries` 可见性放宽、projection/mod.rs
  一行 re-export、branch-E facts 携带、`same_parent` 上提、尾部 refresh 化、
  upper 门取 Ok——均行为保持或由冻结位置强制）。主线程验证：
  `cargo check` exit 0（仅原有 `uuid_mode` 警告）、`cargo fmt --check`
  干净（主线程执行机械 `cargo fmt` 后）。Bug B 未实现。下一步：等用户
  授权后重跑 `overlay/031` + `overlay/038` 验证（Checker 车道；ls3 预期
  保持为 Bug B 依赖失败）。
- **修复验证（2026-08-08，pass_41）：`overlay/038` PASS**——impure 标记
  set/clear 与过滤面断言全部通过。**`overlay/031` 范围内目标 VERIFIED**——
  两次 `ENOTEMPTY` 已消失（branch-A 预扫生效，rm3 lower-backed 发布正确），
  唯一剩余 diff 行是 ls3 `ENOENT`，即验证契约中明确记录的 out-of-scope
  Bug B 依赖（base 视图一致性修复前预期保持 FAIL，不计为本修复失败）。
  无 pass_40 修复批次。证据
  `run_evidence/{overlay031,overlay038}/impure_cleanup_20260808/`；报告
  `components/wave7-xfstests-sequencing/pass_41_wave7_impure_cleanup_checker.md`。
- **031/041 遗留问题登记（2026-08-08，user-directed）：**
  - **031 → Bug B（base-fs↔overlayfs 视图一致性，独立待办）：** pass_40
    后 031 仅剩 ls3 `ENOENT` 一行失败：rm3 发布的 whiteout（字符设备 mode
    0000）在后续 base-fs `rm -rf $upperdir/testdir` 中不可见（overlay 直写
    绕过 base 挂载 VFS 视图），后续挂载 lookup 命中该 whiteout → testdir
    隐藏。同族还包括 031→020 跨用例 `_scratch_mkfs` 残留（陈旧 dirent →
    空目录 rmdir ENOTEMPTY）。修复需三选一：overlay 物理上层写点统一经
    base 挂载 `Path` 层路由 / 加缓存失效接缝 / base 读目录回读盘面——均触碰
    VFS 边界，需用户决策后单开设计任务；修复前每例必须重建镜像。
  - **041 → xino=on 挂载选项回显缺口（接口待办）：** `xino=on` 被内核正确
    解析（`XinoMode::On`，xino 投影在 038 已验证有效），但
    `/proc/self/mounts` 只输出通用标志（`PerMountFlags`），无 fs 特有选项
    （无 Linux `show_options` 等价物），用例 `_fs_options $SCRATCH_DEV |
    grep xino=on` 永远失败 → NOTRUN。修复需在 procfs/VFS 加"导出 fs 挂载
    选项"接缝并让 `MountsFileOps` 打印，属 VFS 边界决策。附带疑点：用例
    grep 按 `/dev/vde` 匹配 /proc/mounts 行，回显修复后需先确认 guest 侧
    行布局（overlay 行 source 是路径而非 /dev/vde）再承诺用例可过。

- **Bug B 解决方向调研（2026-08-08，user-directed；只读）：** "mount 期
  保存所有 layer path" 方案评估——**可行**，但正确定义是"root Path 锚点 +
  沿 dentry 层逐级解析 / 随 binding 携带 dentry 锚定的 Path"，等价于
  Linux（Linux 也不保存全部路径，而是从根经底层 fs 的 VFS 逐步解析）。
  - 可行性证据：mount 期已解析出 layer root `Path`（`resolve_root_path`）；
    `Path = {mount, dentry}` 可 `Clone` 长期持有（先例：
    `ProcessVm::executable_file: Path`）；`Path::{mknod,link,unlink,rmdir,
    rename(..., mode: RenameMode → 支持 Exchange),new_fs_child,create_tmpfile}`
    均走 dentry 层并维护 `DentryChildren`（024 先例已证）；
    `Path::new`/`dentry()` 与 `Dentry::lookup_child` 为 `pub(in
    crate::fs)`，overlayfs 可达。
  - 操作分类（用户确认的理解）：**必须走 Path** = 名称解析
    （`lookup_child`）+ 一切改变目录项集合的操作（whiteout 发布的
    mknod/link、copy-up 的 workdir→upper rename、clear-empty 的 Exchange、
    remove/sweep 的 unlink/rmdir、new_fs_child）；**可留裸 inode** = 已解析
    inode 的内容/元数据（read/write、xattr get/set/remove 含 impure 标记、
    mode/owner/times、resize/sync、overlay 自身 `readdir_at`——后者读 fs
    真值，一致性是"单向"：overlay 写走 dentry 即更新共享树）。
    012 stale-upper 的 fresh-lookup 验证宜顺路走 dentry。
  - 遗留设计成本：`RealObject`/facts 载体加 `Path`（Architect/Designer
    级载体变更）、`DIR → dentry-children` 锁序、dentry 生命周期与
    `revalidate_cached_entry` 对齐、base 挂载先卸的契约；症状 (b)（盘上
    陈旧 dirent）是否纯由 overlay 裸 unlink 造成仍未确诊——若 ext2 驱动
    unlink 自身不合并 dirent，路径路由只能防新污染、不能修旧盘。
  - 结论：作为 Bug B 设计调研的**候选 A**（与缓存失效接缝 / base 读回盘面
    并列），待用户拍板 VFS 边界后开设计任务。

- **Logic-bug repair wave start (2026-08-07, user-directed):** user
  authorized Designer + Creator repair of the five Wave7 FAIL groups and a
  main-agent compile check after implementation (no experiment restart).
  Pass-slicing decision `wave7_logic_bug_repair_20260807` recorded in
  `PASS_SLICING.md`; upstream sources for `overlay/010`, `012`, `013`,
  `014`, `024`, `026` preserved under
  `components/wave7-xfstests-sequencing/run_evidence/upstream_sources/`.
  Designer packet
  `subagent-tasks/wave7_logic_bug_repair/task_designer_wave7_logic_bug_repair_20260807_dispatch.md`
  created; Designer dispatched (or main-agent fallback per the multi-agent V2
  delivery note) to produce the bounded repair addendum under
  `components/wave7_logic_bug_repair/`.
- **Designer acceptance (2026-08-07):** `task_designer_wave7_logic_bug_repair_20260807`
  ACCEPTED structurally. The bounded repair addendum
  (`components/wave7_logic_bug_repair/wave7_logic_bug_repair_designer_spec.md`
  + `_designer_validation.md`) freezes all five objectives: O1 workdir
  residue cleanup (`prepare_workdir` + `remove_work_entries` +
  `WORK_RESIDUE_NAME`, no `ENOTEMPTY` on root residue), O2 readdir opaque
  layer barrier (merge stops after the first opaque layer), O3 `ETXTBSY`
  (VFS `Inode` default-method seam; `exec_write_deny_count` on
  `OverlayInode`; `resize_impl` check before copy-up; deny at exec/init/fork
  and release at `ProcessVm` drop), O4 getxattr `EOPNOTSUPP` for non-`Public`
  names, O5 `translate_stale_upper_enoent` on the three physical-upper
  `ENOENT` sites. No topology/lock change, no ktest surface, no pass slicing.
  Creator packet
  `subagent-tasks/wave7_logic_bug_repair/pass_28_wave7_logic_bug_repair_creator_dispatch.md`
  created; Creator dispatch next (command-free; main agent then verifies the
  target-specific `cargo check` per user direction).
- **Creator pass `pass_28_wave7_logic_bug_repair` (2026-08-07): ACCEPTED with
  one main-agent mechanical continuation.** The Creator (codex-CLI fallback)
  implemented all five objectives per the accepted Designer spec and filed
  `components/wave7_logic_bug_repair/pass_28_wave7_logic_bug_repair_creator.md`
  with the full entity census (WORK_RESIDUE_NAME, remove_work_entries,
  exec_write_deny_count, two Inode trait defaults, two OverlayInode
  overrides, is_write_denied, ProcessVm Drop, translate_stale_upper_enoent).
  The Creator reported one O3 placement deviation (check on the `resize`
  entry instead of `resize_impl` because `copyup/mod.rs` was outside the
  packet write-set) and one mandatory out-of-write-set initializer edit in
  `projection/mod.rs`. The main agent then applied a same-pass mechanical
  continuation to restore the frozen ordering exactly: moved the ETXTBSY
  gate into `copyup/mod.rs::resize_impl` (after the EROFS and MAY_WRITE
  gates, before copy-up), widened `is_write_denied` to
  `pub(in crate::fs::fs_impls::overlayfs)` for the sibling call, widened the
  `exec_write_deny_count` field to `pub(super)` for the inode-cache
  constructor, removed the now-orphaned `DirentCounter` utility (its only
  active consumer was the deleted `is_workdir_empty`), dropped the redundant
  `Inode` imports in `init_proc.rs`/`process_vm/mod.rs`, underscored the
  unused `Vec<String>` visitor params, and switched the deprecated
  `fetch_update` to `try_update`. `cargo fmt --check` on the changed files
  passes.
- **Compile verification (2026-08-07, user-directed, main-agent run):**
  `docker exec -w /root/asterinas codex-asterinas-dev bash -lc 'cd
  /root/asterinas/kernel && cargo check -p aster-kernel --target
  x86_64-unknown-none'` exits 0 with exactly one pre-existing warning
  (`MountPolicy::uuid_mode` dead field in `mount/policy.rs`, untouched by
  this wave). No QEMU/xfstests run was started (user: do not restart the
  experiment). Full `make kernel`/`make check`/xfstests remain unscheduled.
- **VFS revert (2026-08-07, user-directed):** the O3 `ETXTBSY` mechanism was
  removed in full — `Inode::deny_write_access`/`allow_write_access` defaults
  (`fs/vfs/fs_apis/inode.rs`), the `OverlayInode` count field/overrides/
  helper (`projection/inode.rs`), the inode-cache initializer
  (`projection/mod.rs`), the `resize_impl` gate (`copyup/mod.rs`), and the
  exec/fork/drop lifecycle (`process/execve.rs`,
  `process/process/init_proc.rs`, `process/process_vm/mod.rs`). All seven
  files are byte-identical to HEAD. Interface-breaking VFS modifications are
  refused for this wave; `overlay/013`'s `ETXTBSY` remains the documented
  §19c divergence pending a redesign with no VFS interface change.
- **Workdir workspace Designer audit (2026-08-07): ACCEPTED.** User-directed
  revision `task_designer_wave7_workdir_workspace_audit_20260807`:
  `<workdir>/work` becomes the actual staging workspace (Linux
  `OVL_WORKDIR_NAME`/`ofs->workdir` parity). Audit disposed all six usage
  site groups (workspace resolver, mount preparation, capability probes,
  claim surface, staging consumers, carriers) and froze the Rust surface:
  pinned `workdir_workspace: Option<Arc<dyn Inode>>` on
  `UpperWorkdirClaim` + `workdir_workspace()` accessor; `prepare_workdir`
  creates/pins the workspace (residue-clean + recreate, no `ENOTEMPTY`);
  mount order prepare (step 7) → probes against the workspace (step 8) →
  UUID persist (step 9); `workdir_root` resolves the workspace; probes
  renamed `workdir_inode` → `workspace_inode` (logic unchanged);
  `workdir_inode()` kept as the claim surface with `#[expect(dead_code)]`.
  Artifacts:
  `components/wave7_logic_bug_repair/wave7_workdir_workspace_designer_spec.md`
  + `_designer_validation.md`.
- **Workdir workspace Creator pass `pass_29_wave7_workdir_workspace`
  (2026-08-07): ACCEPTED.** Implemented the frozen surface with no
  deviations; report
  `components/wave7_logic_bug_repair/pass_29_wave7_workdir_workspace_creator.md`.
  Main-agent compile verification reran after acceptance:
  `cargo check -p aster-kernel --target x86_64-unknown-none` exits 0 (one
  pre-existing `MountPolicy::uuid_mode` warning); `cargo fmt --check` clean
  on all changed files. No QEMU/xfstests run.
- **Stale-upper TODO annotation (2026-08-07, user-directed):** added a
  `TODO(stale-upper)` doc annotation on `translate_stale_upper_enoent`
  (`dir/remove.rs`) recording that the post-operation `ENOENT`→`ESTALE`
  translation is an indirect approximation; the faithful approach is
  VFS-level dentry verification (fresh upper lookup vs cached upper before
  the physical op, Linux `ovl_matches_upper`), deferred because it would
  require a breaking VFS interface/behavior change that this wave
  intentionally avoids. Comment-only edit; `cargo check` still exits 0.
- **Commit `3299cc8b5` (2026-08-07):** `Fix overlayfs bugs found by simple
  xfstests cases` — 13 Rust files (workdir workspace, readdir opaque
  barrier, xattr `EOPNOTSUPP`, stale-upper `ESTALE` + TODO, `DirentCounter`
  removal). `.agents` records remain uncommitted per protocol.
- **Fixed-case re-run batch (2026-08-07, user-directed; main agent executed
  the Checker lane directly — subagent dispatch unavailable):** serial
  single-case runs of `overlay/024`, `010`, `014`, `026`, `012` on commit
  `3299cc8b5`. Result: `010` **PASS**; `024` **FAIL** (cleanup happens on
  disk but the base mount's stale `DentryChildren` cache still reports
  `work/foo` — prepare uses raw inode ops, bypassing the VFS dentry layer);
  `014` **FAIL** (pre-existing **lowerdir ordering inversion**:
  `normalize_lower_ordering` reverses the colon-split single option, but
  Linux stacks the first path topmost; the whiteout `d` in `lower2/testdir`
  never hides `lower1`'s `d`; secondary: `lower2/testdir` lacks the persisted
  opaque marker); `026`/`012` **FAIL at setup** (`prepare_workdir` creates
  `<workdir>/work` with mode 0o000, so the harness `rm -rf` sweep cannot
  unlink the previous run's leftover workdir temps → `Permission denied`).
  Full report: `components/wave7-xfstests-sequencing/pass_36_wave7_fixed_cases_rerun_checker.md`;
  per-case logs under `run_evidence/<case>/`. Repair batch: (1) workspace
  dir mode 0o700/0o755, (2) lowerdir ordering fix, (3) prepare-time dentry
  invalidation via a non-breaking seam, (4) opaque-marker persistence triage.
- **Workdir mode fix requirement (2026-08-07, user-directed):** when the
  workspace-dir mode repair lands (`prepare_workdir` creating `<workdir>/work`
  with a usable mode instead of `InodeMode::empty()`), the code MUST carry a
  TODO comment explaining the Linux divergence: Linux `ovl_workdir_create`
  creates `work/` with mode 0o000 (`S_IFDIR|0`, clearing inherited bits) and
  relies on `generic_permission`'s directory special-case (CAP_DAC_OVERRIDE
  overrides all DACs for `S_ISDIR`, including the no-exec-bit case), whereas
  this kernel's `check_permission` applies the "exec override requires at
  least one exec bit" rule to directories too, so root cannot traverse or
  unlink inside a 0o000 directory. The workspace therefore uses a usable mode
  (0o700/0o755) instead of replicating 0o000.
- **Prepare-time dentry invalidation divergence requirement (2026-08-07,
  user-directed):** when the prepare-time dentry invalidation repair lands
  (reworking `prepare_workdir` cleanup onto the mount-time `Path` API), the
  cleanup site MUST carry an explicit TODO/comment stating the inconsistency
  with Linux behavior: the current raw-inode cleanup bypasses the VFS dentry
  layer, so the base mount's stale `DentryChildren` cache still reports the
  removed entries (e.g. `work/foo`), whereas Linux performs the workdir
  cleanup through the VFS/upper-fs dentry layer so the cached directory view
  stays coherent; the rework therefore routes cleanup through the mount-time
  `Path` API to update the base view's `DentryChildren` with zero VFS
  interface change.
- **Fixed-case rerun Designer addendum (2026-08-07, in-thread Designer
  execution; no subagent per user direction):**
  `task_designer_wave7_fixed_case_rerun_20260807` ACCEPTED structurally.
  Artifacts:
  `components/wave7_logic_bug_repair/wave7_fixed_case_rerun_designer_spec.md`
  + `_designer_validation.md`. Frozen surface: (R2-A) delete
  `normalize_lower_ordering` and make `lower_dirs`/`lowers` topmost-first
  with Linux parity (`options.rs` doc corrections only; `layers.rs` loop
  consumes the parsed list directly); (R2-B) new `WORKDIR_MODE =
  InodeMode::from_bits_truncate(0o700)` const with the frozen divergence
  TODO; (R2-C) `prepare_workdir(&mut self, workdir_path: &Path)` — the
  visible `work` name is removed/recreated through
  `Path::rmdir`/`Path::unlink`/`Path::new_fs_child` so the base view's
  `DentryChildren` is updated, `remove_work_entries` raw recursion retained
  (residue dentry is discarded wholesale; rationale frozen in the spec),
  divergence TODO at the cleanup site, zero VFS interface change;
  `build.rs` step 7 passes `&workdir_path`. No topology/lock change, no
  ktest surface, no pass slicing, no production `.rs` written. Out of scope
  per handoff: 014 opaque-marker/clear-empty triage and `overlay/013`
  `ETXTBSY`.
- **Fixed-case rerun Creator pass `pass_37_wave7_fixed_case_rerun`
  (2026-08-07, in-thread; no subagent per user direction): ACCEPTED.**
  Implemented the accepted Designer addendum exactly (no deviations):
  `normalize_lower_ordering` deleted and `lower_dirs`/`lowers` topmost-first
  (`options.rs`/`layers.rs` docs corrected); `WORKDIR_MODE` const (0o700)
  with the frozen divergence doc; `prepare_workdir(&mut self,
  workdir_path: &Path)` removing/recreating the visible `work` name through
  `Path::rmdir`/`Path::unlink`/`Path::new_fs_child` with
  `TODO(workdir-cleanup-vfs-parity)` and `TODO(workdir-mode)` comments;
  `build.rs` step 7 passes `&workdir_path`. `remove_work_entries` raw
  recursion retained per the frozen rationale. Report:
  `components/wave7_logic_bug_repair/pass_37_wave7_fixed_case_rerun_creator.md`.
  Pass-slicing record added to `PASS_SLICING.md`.
- **Fixed-case rerun compile verification (2026-08-07, user-directed,
  main-agent run):** `cargo check -p aster-kernel --target
  x86_64-unknown-none` exits 0 (only the pre-existing
  `MountPolicy::uuid_mode` dead-field warning, untouched); `cargo fmt
  --check` on the four changed files exits 0. No QEMU/xfstests run (user:
  wait for instruction).
- **Fixed-case rerun xfstests (2026-08-07, user-directed; individual runs,
  each case exactly once; images rebuilt fresh; 014 last):** all five cases
  **PASS** — `overlay/024` (workdir cleanup + base-view coherence),
  `overlay/010` (whiteout residue), `overlay/026` (setup hygiene + xattr),
  `overlay/012` (setup hygiene + stale-upper), `overlay/014` (lowerdir
  ordering parity: `lowerdir2:lower1` now stacks `lowerdir2` topmost; the
  whiteout `d` is hidden and the post-copy-up/remount merged readdir is
  clean). The TEST/SCRATCH images were deleted and freshly recreated (8 GiB
  ext2, new UUIDs; SHA-256 recorded in the report) to clear the previous 014
  workdir pollution; the reused-image path also validated the cross-run
  `_scratch_mkfs` cleanup (mode fix) with no `Permission denied`. Temporary
  single-case runlist `wave7-single.list` created per case and deleted after
  the batch. Report:
  `components/wave7-xfstests-sequencing/pass_38_wave7_fixed_case_rerun_checker.md`;
  per-case logs under `run_evidence/<case>/rerun_20260807/`.
- **Post-batch cleanup (2026-08-07, user-directed):** the rebuilt
  TEST/SCRATCH images were **deleted** after evidence capture (Docker-over-WSL
  VHDX host-space concern); the receipt retains only their size (8589934592
  bytes each) and SHA-256 (recorded in `pass_38`). The four fixed-case-rerun
  production files (`mount/{options,layers,claims,build}.rs`) were **amended
  into the latest commit**, and per explicit user direction the three tracked
  `.agents` records (`PASS_SLICING.md` and both live main-agent handoffs)
  were amended in as well (same message "Fix overlayfs bugs found by simple
  xfstests cases"). Historical references to `3299cc8b5`/`e396c55cb` in this
  handoff describe pre-final-amend states; see `git log -1` for the current
  HEAD hash.
- **`overlay/014` sequencing decision (2026-08-07, user-directed):** fix the
  lowerdir layer-order inversion FIRST
  (`normalize_lower_ordering`/`mount/options.rs`: for a single colon-joined
  `lowerdir=` option the first path is the topmost layer per Linux docs; the
  current reverse makes `lower1` topmost and defeats both per-name whiteout
  lookup and the readdir opaque barrier), then RE-RUN `overlay/014` to
  observe whether new issues surface (the readdir barrier itself is believed
  correct; the absent persisted opaque marker on the recreated dir and the
  mount-1 clear-empty failure are separate triage items to be revisited only
  after the ordering fix).
- **单纯用例批次（2026-08-07）：** 按用户指示顺序执行全部 single-purpose
  用例（22 例），综合/stress 用例未测。结果矩阵与逐例证据：
  `wave7_simple_cases_batch_summary.md`；日志按用例归档于
  `run_evidence/<case>/`。
  - PASS（8）：`009`（default_permissions）、`002`（fsync）、`007`
    （getcwd）、`016`（ro/rw fd）、`003`（whiteout sanity）、`006`
    （rename whiteout）、`011`（opaque xattr 隐藏）、`039`（relatime）。
  - FAIL（6）：
    - `024`/`010`：workdir 根空检查过严（`prepare_workdir` 返回
      `ENOTEMPTY`，Linux 应清理 `work/` 子目录残留）；
    - `013`：truncate 运行中二进制未返回 `ETXTBSY`；
    - `012`：stale upper dentry 删除返回 `ENOENT`（预期 `ESTALE`）；
    - `026`：`trusted.overlay.*` getxattr 返回 `ENODATA`（预期
      `EOPNOTSUPP`）；
    - `014`：readdir 合并缺 lower 层 opaque 屏障（明确缺口：
      `readdir_index.rs::readdir_sequence` 只对 upper 检查 opaque，lower
      层屏障未实现，lowerdir2 opaque 未挡住 lowerdir1 的 `d`；copy-up
      xattr 过滤本身正确）。clear-empty 持久化问题暂不讨论。
  - NOTRUN（8）：`035`/`040`/`027` chattr 缺失；`004`/`008`/`015`/`025`
    `fsgqa` 用户缺失；`023` `chacl` 缺失。
- **Run 4 result (2026-08-07, `run_001_20260807_1139`):** `overlay/002`
  (new order 4) — **PASS** (merged-directory write + fsync; `Passed all 1
  tests`). Evidence: `run_evidence/overlay002/`; report
  `pass_34_wave7_overlay002_checker.md`.
- **Run 3 result (2026-08-07, `run_001_20260807_1137`):** `overlay/009`
  (new order 3) via temporary single-case runlist `wave7-single.list`,
  `XFSTESTS_DISK_SIZE=8G`, reused images. Result: **PASS**
  (`default_permissions` mount + read + clean teardown; `Passed all 1
  tests`). First behavioral PASS of Wave7. Evidence:
  `.agents/components/wave7-xfstests-sequencing/run_evidence/overlay009/`;
  report `pass_33_wave7_overlay009_checker.md`.
- **Run 2 result (2026-08-07, `run_001_20260807_1136`):** `overlay/024`
  (new order 2) — **FAIL** (first behavioral failure). Guest mount failed:
  `fsconfig() failed: Directory not empty` (`ENOTEMPTY`). Root cause located:
  `UpperWorkdirClaim::prepare_workdir()` (`mount/claims.rs:298-303`) requires
  the workdir root to be empty, while Linux overlay semantics keep the
  internal work under `<workdir>/work` and clean residue there at mount
  (upstream `tests/overlay/024` pre-creates `work/foo` and expects it gone
  after mount). Repair batch recorded in
  `pass_32_wave7_overlay024_checker.md`; route to the meso-01 workdir owner.
- **Run 2 result (2026-08-07, `run_001_20260807_1130`):** `overlay/035`
  (new order 1) via temporary single-case runlist `wave7-single.list`,
  `XFSTESTS_DISK_SIZE=8G`, fresh 8 GiB ext2 TEST/SCRATCH images, RELEASE=1
  MEM=12G. Kernel/initramfs built; guest reached xfstests with
  `FSTYP -- overlay`. Result: `[not run] file system doesn't support
  chattr +i`. No panic, hang, or filesystem behavior failure. Upstream
  source confirms the theme (read-only mount cases; one scenario needs
  `chattr +i` on the workdir), so the handoff theme stands. Evidence and
  report under `.agents/components/wave7-xfstests-sequencing/`
  (`pass_31_wave7_overlay035_checker.md`, `run_evidence/`); temporary
  runlist deleted; no QEMU remains.
- **Capability gap (2026-08-07):** the `chattr +i` prerequisite failure
  exposes that the current kernel has no fileattr/`FS_IOC_*` implementation
  (`rg` in `kernel/src/fs/` found none). This gates `overlay/040`, `027`,
  and `078` later in the order; those rows will NOTRUN at the same gate until
  fileattr support exists or the scope decision changes.
- **Table reconciliation (2026-08-07, user-directed):** Cleaned and re-sorted
  the Section 2 obligation table after reading the upstream suite source for
  all 38 cases. Most prior "Current-scope purpose" rows were mis-attributed
  (e.g. `overlay/002` is an fsync crash regression, not lookup; `overlay/004`
  is mode-bit copy-up, not opaque/whiteout lookup; `overlay/005` is a
  copy-up-error memleak regression needing loop/XFS, not merged readdir;
  `overlay/007` is getcwd-after-failed-rmdir, not readdir/`d_ino`;
  `overlay/019` is fsstress, not stat/readdir consistency; `overlay/021` is
  concurrent copy-up bombs, not writable-mount setup; `overlay/032`-`034`,
  `037`, `042` are `index=on` hardlink/nlink tests, not rename/SGID/xino;
  `overlay/039`/`040` are atime/ioctl regressions, not mmap/fsync
  delegation; `overlay/078` is fileattr copy-up, not UUID observation).
  New order: 32 schedulable cases by foundation tier then simplicity, plus
  six classified not schedulable now (Section 2.2). Heavy/space cases
  (`overlay/001` ≥8 GiB, `overlay/021` ~16 GiB, `overlay/019` stress) moved
  to the tail.
- **Dispatch / execution note (2026-08-07):** Two Checker subagents were
  spawned with `fork_turns="none"` per protocol; neither received its task
  content correctly. Per explicit user direction, both were interrupted and
  the main agent executed the authorized Checker lane directly.
- **Run 1 result (2026-08-07, `run_001_20260807_1120`):** `overlay/001`
  via temporary single-case runlist `wave7-single.list`, `XFSTESTS_DISK_SIZE=8G`,
  fresh 8 GiB ext2 TEST/SCRATCH images, RELEASE=1 MEM=12G. Kernel/initramfs
  built; QEMU booted; guest reached xfstests with `FSTYP -- overlay`.
  Result: `[not run] This test requires at least 8GB free on
  /opt/xfstests/scratch to run` (scratch image free ≈ 7.85 GiB). No panic,
  hang, or filesystem behavior failure. Evidence preserved under
  `.agents/components/wave7-xfstests-sequencing/run_evidence/`; report at
  `.agents/components/wave7-xfstests-sequencing/pass_30_wave7_overlay001_checker.md`;
  temporary runlist deleted; no QEMU remains.
- **Theme discrepancy (escalation, 2026-08-07):** the guest upstream source
  (`tests/overlay/001`) proves the case is a copy-up test for files <, =,
  > 4 GiB (`ovl: use O_LARGEFILE in ovl_copy_up()` regression), gated by
  `_require_fs_space $OVL_BASE_SCRATCH_MNT $((4*1024*1024*2 + 8))`. The
  Wave7 ordering and the meso-01 contract row calling it "mount-option
  validation / root-stat baseline" are not supported by the suite source.
- **Wave7 start decision (2026-08-07):** User explicitly authorized starting
  the xfstests sequencing recorded in Section 2.
- **Dispatch (2026-08-07):** Checker packet
  `pass_30_wave7_overlay001_checker_dispatch.md` created under
  `.agents/subagent-tasks/wave7-xfstests-sequencing/` and dispatched for
  `overlay/001` only, with evidence destination
  `.agents/components/wave7-xfstests-sequencing/run_evidence/`. The packet
  authorizes the verified overlay xfstests lane (`$ovfs-checker`) with a
  temporary single-case runlist
  (`test/initramfs/src/conformance/xfstests/overlay/run_list/wave7-single.list`),
  a fresh 8 GiB TEST/SCRATCH image pair, theme confirmation from the upstream
  suite source, and the exact scope exclusions in Section 3.
- **User-directed mechanical deferred-expect cleanup (2026-08-06):** Removed
  the unused claim and UUID accessors from `mount/claims.rs` and
  `mount/policy.rs`: `InodeClaimGuard::token`,
  `UpperWorkdirClaim::{has_exclusive_claim,upper_inode,identity}`, and
  `MountPolicy::uuid_mode`; and removed the now-unreferenced
  `OverlayInuseSlot::is_claimed_by` query. `UpperWorkdirClaim` no longer
  duplicates the upper filesystem `Arc`; `selected_real_fs` now reads the
  canonical upper filesystem from `OverlayLayerStack`. This is ownership
  cleanup only: claim acquisition, release, lifecycle semantics, and
  upper/lower selection remain unchanged. The nine retained `dead_code`
  expectations are the two VFS
  unmount/shutdown seams, two VFS scoped-credential seams, and five VFS
  writer/freeze seams; each now carries a concrete TODO and VFS-specific
  reason. `cargo fmt --check` and `git diff --check` passed. The usual
  workspace Clippy command could not start because this host lacks
  `cargo-osdk`; a standalone `cargo check -p aster-kernel` with an isolated
  target directory instead failed in pre-existing host configuration before
  compiling the kernel crate, because OSDK-provided architecture dependencies
  were absent. No Wave7 runtime command ran.
- **Dispatches Sent:** None.
- **Commands Run:** None for Wave7.
- **Acceptance Outcomes:** None. This is a scheduling record, not an
  xfstests result or an integration acceptance.
- **Escalations / Deadlocks:** None. The UUID mapping discrepancy is recorded
  above as a pre-start reconciliation item, not a runtime failure.

- **Bug B Path 设计任务派发与验收（2026-08-09，user-directed；V2 lane）：**
  packet `subagent-tasks/wave7_bug_b_path_design/task_designer_wave7_bug_b_path_20260809_dispatch.md`
  （§3 清单 9 项 + 用户补充的 unmount 双向研讨环节 + `~/linux` 源码树提醒）；
  用户贴派发轮后按协议 spawn `fork_turns="1"`。Designer 完成并产出：
  `components/wave7_bug_b_path_design/wave7_bug_b_path_designer_spec.md`（45.9
  KB）与 `_designer_validation.md`（11.4 KB）。主代理结构性验收 PASS：模板小节
  齐全、清单 1-9 逐项处置、micro ID 对照 inventory（P1-26/P1-36/P1-01..07/
  P1-34/P0-14/P1-31/P1-33 + 邻接表）、抽查 file:line 引用一致（`Path::new`/
  `dentry()`/`lookup_child` 可达性、`Path::link/rename` same-Mount EXDEV、
  `Mount::do_unmount` 无引用门、`OverlayLayer`/`RealObject`/`WorkdirTemp` 现状）。
  冻结面摘要：`OverlayLayer.root_path`；`RealObject.real_path: Option<Path>`
  （None 仅 readdir `..`，accessor 检查返回）；`WorkdirTemp.path`/
  `WhiteoutHandle.path`；`upper_parent_path()`/`workdir_root_path()`/
  `workdir_workspace_path()` 三个 owner-private 包装；≈20 个裸 inode 写点改走
  `Path` 方法（lookup_child/mknod/link/unlink/rmdir/rename(Exchange)/
  new_fs_child），内容元数据类（read/write、xattr 含 impure、mode/times、
  overlay `readdir_at`）保留裸 inode；unmount 双向：overlay 先卸 = 现有
  last-Drop RAII（Asterinas 无 VFS unmount 回调，加回调属 VFS 接口改动、
  本 packet 禁止），base 先卸 = 「实际无所谓就不管」（`Mount::do_unmount` 无
  引用/EBUSY 门 + 无 mount-liveness API，持有引用对卸载成功与后续 `Path`
  有效性均无差别），不设计失效机制；新增 mount 期 same-`Mount` `EINVAL`
  校验（Linux super.c:806-811 对齐，`no upstream coverage` 已记录）。零 VFS
  接口改动、零新锁域、零 ktest、无 Creator 切片。验证契约：`overlay/031`
  direct（ls3 + 跨用例残留；rm1/2/3 回归守卫）+ 邻接 010/024/012/003/006/
  011/029/077/038；020/041/013 明确越界。下一步：待用户授权 Creator pass
  切片（届时先记 `PASS_SLICING.md`，再按 V2 lane 派发）。

- **pass_42 切片与 Phase A 派发准备（2026-08-09，user-directed）：**
  `PASS_SLICING.md` 已记录 `pass_42_wave7_bug_b_path_repair`（一 pass 三阶段：
  Phase A 载体/锚点/查找路由 — `mount/{layers,claims}.rs`,
  `projection/{entry,inode}.rs`, `dir/mod.rs` 仅加 `upper_parent_path()`；
  Phase B workdir temp + copy-up/link 腿 — `copyup/{workdir,promote}.rs`,
  `dir/link.rs` + create/whiteout/remove 三处 `create_workdir_temp` 传参机械
  替换；Phase C dir 系语义扫尾 — `dir/{create,whiteout,remove,rename,mod}.rs`，
  删 `upper_parent`/`workdir_root`）。执行纪律（用户确认）：每阶段 = V2 派发轮
  + spawn `fork_turns="1"`（task_id 带 `_phase_a/b/c` 后缀，B/C 携带
  Continuation / Parent Task 指针）；Creator command-free、中途不编译；阶段
  验收 = 纯 diff 审查；Phase C 后主代理跑目标 `cargo check` 并只修机械编译
  错误（语义/缺失面错误退回 Creator 修复轮）；Git：Phase A 新建 commit、
  B/C amend 同一 commit，`git diff HEAD` 单阶段呈现；`.agents` 记录不提交。
  Phase A packet：
  `subagent-tasks/wave7_bug_b_path_design/task_creator_wave7_bug_b_path_20260809_phase_a_dispatch.md`
  （含冻结清单、实体普查期望、禁止面）。派发轮已生成，待用户贴出。

- **Phase A 执行与验收（2026-08-09，V2 lane；ACCEPTED）：**
  Creator `task_creator_wave7_bug_b_path_20260809_phase_a` 完成，产出报告
  `components/wave7_bug_b_path_design/pass_42_wave7_bug_b_path_phase_a_creator.md`
  + 5 个生产 `.rs`（`mount/{layers,claims}.rs`、`projection/{entry,inode}.rs`、
  `dir/mod.rs`）。主代理结构性 diff 验收 PASS：`OverlayLayer.root_path` 锚点、
  `UpperWorkdirClaim.workdir_workspace_path`（与 `workdir_workspace` 同语句写入）、
  `validate_pair` same-`Mount` `EINVAL`（Linux `ovl_get_workdir` 依据）、
  `RealObject.real_path` + `with_path`/`real_path()`（`Err(EIO)` on None）、
  `lookup_in_layers` 上/下层改 `Dentry::lookup_child` + `Path::new`、
  `new_root` 经 `layer.root_path`、`upper_parent_path()` 新增（`upper_parent`/
  `link_impl` 保留）；实体普查 = 3 字段 + 4 方法 + 0 新 struct/enum/中间体 +
  0 删除，两个暂无可调用者的新访问器按 Wave6 先例加 `#[expect(dead_code)]`
  （记录为 incidental，B/C 接线后移除）；禁止面零改动（`copyup/`、
  `dir/{create,link,remove,rename,whiteout}.rs`、`readdir_index.rs`、
  `metadata_security/`、`mount/build.rs` 均无 diff）。中途未编译（用户纪律）。
  Git（用户纪律）：Phase A 新建 commit `f68ecdcec`（仅 5 个生产 `.rs`，
  `.agents` 记录未提交）；Phase B/C 将 amend 该 commit。

- **Phase B 执行与验收（2026-08-09，V2 lane continuation；ACCEPTED）：**
  Creator `task_creator_wave7_bug_b_path_20260809_phase_b` 完成，产出报告
  `components/wave7_bug_b_path_design/pass_42_wave7_bug_b_path_phase_b_creator.md`
  + 6 个生产 `.rs`（`copyup/{workdir,promote}.rs`、`dir/link.rs`，
  `dir/{create,whiteout,remove}.rs` 仅 `create_workdir_temp` 传参机械替换）。
  主代理结构性 diff 验收 PASS：`WorkdirTempRequest::Link` 载荷改 `Path`、
  `WorkdirTemp.inode` → `path`（`inode()` 派生、`into_parts() -> (String,
  Path)`）、`create_in`/`create_workdir_temp`/`cleanup_workdir_temp` 转
  `&Path`、新增 `OverlayFs`/`OverlayInode::workdir_root_path`（旧 inode
  访问器保留给 create/whiteout/remove 余下消费者）、promote 四臂
  `workdir_path.rename(&temp_name, &upper_dir_path, name, Replace)` +
  `upper_real_object` 经 `lookup_child` + `with_path`、`link_source` 返回
  `Result<Path>`、`link_over_whiteout` 走 `Path::rename`；whiteout 站点
  第二参数按原语义传 `&workdir_path`（temp 名以 workdir ino 作种，doc 明示
  非 (parent,name) 属主），create/remove 传 `&upper_parent_path`，行为保持。
  实体普查：0 新 struct/enum/命名中间体、0 删除；`verify_upper_target` 参数
  `&Path`、两处局部绑定、whiteout 局部改名记为 incidental（行为保持）；
  禁止面（`dir/{rename,mod}.rs`、`readdir_index.rs`、`metadata_security/`、
  `mount/build.rs`）零改动。已知三个中间态断点（whiteout `into_parts`→
  `WhiteoutHandle.inode`、create `RealObject::new` 收 `Path` temp、`link_impl`
  直链分支收 `Path`）为 Phase C 消解面，Creator 报告 §5 明示；中途未编译
  （用户纪律）。Git：amend 进 Phase A commit → `c5d7e36db`（11 文件
  +369/-143，`.agents` 记录未提交）。

- **Phase C 执行、首次编译失败与 lookup 修订决策（2026-08-09，V2 lane
  continuation；compile BLOCKED on lookup_child）：**
  Creator `task_creator_wave7_bug_b_path_20260809_phase_c` 完成，产出报告
  `components/wave7_bug_b_path_design/pass_42_wave7_bug_b_path_phase_c_creator.md`
  + 8 个生产 `.rs`（`dir/{mod,create,whiteout,remove,rename}.rs`、
  `copyup/{workdir,promote}.rs` 删 `workdir_root`、`mount/claims.rs` 移除
  expect）。主代理结构性 diff 验收 PASS：`link_impl` 走 `Path::link`、
  create 两臂 `with_path`、`WhiteoutHandle.path`、publish/sweep/clear-empty/
  Exchange/重观测全走 `Path`、三访问器删除 + 两 expect 移除；完成不变量
  （除 readdir `..` 外无裸 inode 命名空间变更）。随后容器内首次编译
  （Checker 已验证方式，主代理亲自执行）→ **8 个 E0599 全部是
  `dentry().lookup_child` 不可达**：`lookup_child` 定义在 `impl DirDentry`
  （dentry.rs:398-953）而非 `Dentry`，唯一构造器 `as_dir_dentry_or_err`
  与 `DirDentry` 均为 `pub(super)`，`Dentry::new` 私有；探针证实
  `pub(crate)` 放宽也不可调用（非可见性问题，是接收者类型错误）。
  只读调研发现可达替代：**`PathResolver::lookup_at_path(&base_path, name)`
  （resolver.rs:555，`pub`）** 内部即 `as_dir_dentry_or_err → lookup_child →
  Path::new → get_top_path`，返回 dentry 锚定 `Path`，零 VFS 改动；resolver
  经 `current_thread!().as_posix_thread().read_fs().resolver().read()` 获取
  （procfs maps.rs:99 先例）。语义差异 3 项（MAY_EXEC/EACCES、dot/dotdot、
  get_top_path 挂载点解析）均更贴近 base 视图；新关注点 = overlay Inode
  方法内获取 resolver 读锁与 VFS 解析路径同线程重入的锁序，待 Designer
  确认。证据：`components/wave7_bug_b_path_design/compile_failure_lookup_child_20260809.md`。
  决策（user-directed）：Phase C 现状 amend 进 commit → `4830cd007`（12
  文件 +582/-314）；派发 bounded Designer 修订
  `task_designer_wave7_bug_b_lookup_revision_20260809` 确认 lookup 替换面
  （8 处）与锁序/MAY_EXEC，验收后 Creator 机械替换 8 处再编译。

- **lookup 修订 Designer 派发与阻断性升级（2026-08-09，V2 lane；
  ESCALATED — 非正常冻结）：**
  Designer `task_designer_wave7_bug_b_lookup_revision_20260809` 完成，产出
  `components/wave7_bug_b_path_design/wave7_bug_b_lookup_revision_designer_spec.md`
  + `_designer_validation.md`。结论：`PathResolver::lookup_at_path`
  （resolver.rs:555，`pub`）机制确认可用（内部即 as_dir_dentry_or_err →
  lookup_child → Path::new → get_top_path，返回 dentry 锚定 `Path`，零 VFS
  接口改动），但**锁序裁定为可证明的自死锁**：
  `sys_chdir`（chdir.rs:21-27）/`sys_chroot`（chroot.rs:17-24）/
  `sys_pivot_root`（pivot_root.rs:34）在持有 `resolver().write()` 期间执行
  `lookup`；路径穿过 overlay 时经 lookup_child → OverlayInode::lookup →
  lookup_in_layers 进入 8 个替换点，嵌套 `resolver().read()` 因
  `try_read` 检测本线程 WRITER 位失败（ostd rwmutex.rs:146-153）、
  `wait_until` 永久睡眠（wait.rs:69-82）——含 `cd $SCRATCH_MNT` 这类
  xfstests 核心操作。排队 writer 不阻塞读者（reader-barging）非死锁源；
  活跃同线程 writer 才是。无 overlay 本地规避（try_read 报错破坏
  chdir/chroot/pivot_root 经 overlay；raw inode 无法构建 dentry 锚定载体）。
  8 站点替换面以**条件冻结**记录（统一 `lookup_at_path` 表达式，纯读 3 处
  亦统一走 lookup_at_path——理由：sweep 重观测清除陈旧负缓存、HiddenEvidence
  观测刚发布名字需与 base 视图一致；raw 读会读到 fs 真值而 base 缓存仍陈旧，
  属对 §5.4 的回归）。MAY_EXEC 裁定：upper 臂冗余等价、lower 臂新增窄
  EACCES 接受并记录；dot/dotdot/NAME_MAX/get_top_path 无回归。主代理核实
  证据属实（三个 syscall 持写锁解析 + rwmutex 同线程 writer 位）。解除
  blocker 的两个方向（Designer §6 给出）：(A) 三个 syscall 改为 read 解析
  + write 仅做状态变更；(B) VFS 暴露可达的 dentry 产出 API——其中最小形态
  即此前发现的 2 行可见性放宽（`DirDentry` + `as_dir_dentry_or_err`
  `pub(super)` → `pub(in crate::fs)`，零行为、无 resolver 锁、无死锁）。
  待用户决策。

- **可见性放宽落地 + overlay/031 单次实测 PASS（2026-08-09，user-directed；
  亲自执行，无 subagent）：**
  VFS 2 行放宽（`DirDentry` + `Dentry::as_dir_dentry_or_err`
  `pub(super)` → `pub(in crate::fs)`，dentry.rs:365/:394）+ 8 处机械改
  `path.dentry().as_dir_dentry_or_err()?.lookup_child(name)`（entry.rs ×2、
  workdir.rs、promote.rs、whiteout.rs、remove.rs ×2、rename.rs）。容器
  `cargo check -p aster-kernel --target x86_64-unknown-none` 通过（仅预存
  `MountPolicy::uuid_mode` 警告）；amend → `e3da18bd9`（13 文件
  +584/-316）。随后用记录中的复现命令（pass_41 §3）**只跑一次**
  `overlay/031`（临时 `wave7-single.list` + 全新 8 GiB ext2 镜像，
  RELEASE=1 MEM=12G）：**PASS** —— `Ran: overlay/031 / Passed all 1
  tests / All conformance tests passed`。即 Bug B 的 ls3 缺口（base 视图
  一致性）在 pass_42 + 可见性放宽后**已修复**。证据归档
  `run_evidence/overlay031/pass42_visibility_fix_20260809/{qemu.log,
  qemu-serial.log}`；临时 runlist 与生成镜像已按卫生要求删除。另执行了
  只读调研构造零 VFS 改动的完整 Resolver 方案，结论记录于
  `components/wave7_bug_b_path_design/resolver_zero_vfs_research_20260809.md`
  （见该文件；核心：raw lookup + 每对象路径链 + 变更时从层锚点经
  `lookup_at_path` 逐级解析父路径，可完全避开 resolver 写锁死锁，但需
  Designer 级载体重做与 rename/失效链同步确认，成本远高于 2 行放宽）。

- **未测四例补齐（2026-08-09，user-directed；亲自执行，无 subagent；
  单例 runlist + 全新镜像，每例一次，10 分钟上限无 HANG）：**
  `overlay/078`（8G）→ **NOTRUN**：`file system doesn't support chattr
  +ASai`（内核无 fileattr，符合预期）。
  `overlay/001`（16G 重建镜像）→ **FAIL（内核 panic）**：
  `Uncaught panic: called Result::unwrap() on an Err value: NoMemory at
  kernel/comps/block/src/bio.rs:429`，发生于 >4GiB 大文件 copy-up 期间
  （guest 45s 处），QEMU 未超时——真实内核缺陷（block BIO 层对 NoMemory
  直接 unwrap）。
  `overlay/021`（16G）→ **FAIL**：并发 copy-up 炸弹，输出不匹配——
  `ovl-lower/arena/p{0..3}/*` 大量 `No such file or directory`、`find:
  'p3'/'p2' No such file or directory`；`Failed 1 of 1 tests`。无 panic、
  非 hang——疑似并发 copy-up（CUL）正确性缺陷，待后续归因。
  `overlay/019`（16G，fsstress）→ **PASS**：`Passed all 1 tests`。
  证据：`run_evidence/{overlay078,overlay001,overlay021,overlay019}/
  untested_four_20260809/{qemu.log,qemu-serial.log}`；临时 runlist 与生成
  镜像已删除。Wave7 台账更新（当时口径）：**PASS 18 / NOTRUN 11 /
  FAIL 2（001、021）/ 013 分歧 1 = 32 可调度全部有结果**，无遗留未测。
  （2026-08-10 注：此 PASS 18 为候选增补前口径；063/022 候选 PASS 后累计
  20，见 §8.2/§8.5。）

## 5. Explicit Agent-Level Decisions

1. Wave7 is started per explicit user authorization. The first run
   (`overlay/001`) is complete with `NOTRUN` (scratch-space prerequisite), and
   the Section 2 table has been cleaned and re-sorted from the upstream suite
   source. No behavioral evidence, acceptance, repair routing, or
   `legacy_fs.rs` deletion exists.
2. Only complete xfstests cases whose intended current-scope behavior can be
   passed as a whole are ordered. Mixed cases with deferred required assertions
   are excluded rather than counted as partial success.
3. Six previously listed cases (`overlay/005`, `032`, `033`, `034`, `037`,
   `042`) are not schedulable in the current lane because they require a
   loop/XFS backing lane or the deferred `index=on` feature; they are recorded
   in Section 2.2 and excluded until a scope decision changes that.
4. Creating this handoff did not itself authorize test, harness,
   `legacy_fs.rs`, production, VFS, Designer, Creator, or Reviewer work. The
   separately user-directed mechanical cleanup is recorded in Section 4 and
   does not authorize Wave7 implementation or runtime work.
5. A future runtime packet must retain xfstests as the sole validation lane;
   no ktest or filesystem-local substitute is permitted.
6. **V2 dispatch delivery contract adopted (user-confirmed 2026-08-08):** the
   User Dispatch Turn (latest user turn) + `fork_turns="1"` is the only
   subagent content channel on this platform; spawn payload, NEW_TASK header,
   followup/send messages, and goal text are not channels. Normative text:
   `PROTOCOL.md` §1.3 and
   `protocol/templates/user_dispatch_turn_TEMPLATE.md`. Mechanism evidence and
   smoke receipt: §4.

## 6. Next Actions for the Next Thread (CRITICAL)

**2026-08-10 NEXT ACTION → 已完成（2026-08-10 下午）：`pass_43_wave7_cache_consistency`
Creator pass 三阶段全部落地并 compile-accepted。** 设计产物：
`components/wave7_cache_consistency_design/wave7_cache_consistency_designer_spec.md`
+ `_designer_validation.md`（Change 1 BindingCache verified memo；Change 2
get_or_create 同对象校验+陈旧替换；Change 3 alias_key F1/F2 位移细化；
Change 4 invalidate_parent；Change 5 readdir 边界 A；零 VFS、无新锁域、无 ktest）。
按用户指示（2026-08-10）做 **一 pass 三阶段**（pass 号不增长；同一 Creator
内部 Phase A/B/C，B/C `--amend` Phase A commit，最终一个 commit
`7d6b5c960`，6 文件 +284/-75）：Phase A = 身份判定/memo 验证原语
（`projection/inode.rs` `same_visible_identity` + `contains_real_inode`；
`projection/binding_cache.rs` `matches_truth` + `is_same_negative` +
`invalidate_parent`；5 个新方法带 `#[expect(dead_code)]` 待接线，零调用点）；
Phase B = 载体缓存校验与失效接线（`inode_cache.rs` `get_or_create` 3 参 +
`alias_key` 4 参 F1/F2、`inode.rs` `replace_facts`/`new_root`、`mod.rs`
`project_inode`、`promote.rs` 一行传参）；Phase C = `lookup_binding` memo 化
（无条件层扫描 + `matches_truth` verify-then-serve）+ `readdir_index.rs`
文档（Change 5）+ 剩余 dead-code 解除。每阶段结构 diff 验收通过后
commit/amend（Phase A `fc8507a2a` → B amend `dac6c65da` → C amend
`deda45635` → 编译机械修复 amend `7d6b5c960`）。派发走 **Direct Spawn Lane**
（PROTOCOL §1.3 优先项，平台验证；三个 Creator：Godel/Franklin/Sagan）。

**2026-08-10 编译门（主代理执行）：** 容器
`cargo check -p aster-kernel --target x86_64-unknown-none` **PASSED（0 错误，
10s）**。机械修复（Wave5/pass_42 先例）：移除 `HiddenEvidence::{
layer_index,real_inode}` 与 `NegativeBinding::{HiddenByWhiteout,
HiddenByOpaque}` 载荷上 4 个已失效的 `#[expect(dead_code, reason=...)]`
（这些字段现被 `is_same_negative` 真实读取）；`Binding::matches_truth`
可见性收窄为 `pub(super)`（其 `&LayerLookup` 参数为 projection 内部类型，
spec §4.4 允许 projection 内部方法用 `pub(super)`）。剩余警告 = 预存
`MountPolicy::uuid_mode`（不属本 pass）。Creator 报告：
`components/wave7_cache_consistency_design/pass_43_wave7_cache_consistency_
{phase_a,phase_b,phase_c}_creator.md`；packets：
`subagent-tasks/wave7_cache_consistency_design/task_creator_wave7_cache_
consistency_20260810_phase_{a,b,c}_dispatch.md`。

**NEXT ACTION（下一线程，待用户授权；2026-08-10 修正回归范围）**：派发
pass_43 的 meso-integration Checker（`$ovfs-checker` lane），按
`wave7_cache_consistency_designer_validation.md` §2 跑**可调度回归批次**
（新鲜镜像、每例一个 QEMU；expected `PASS`，其中 031 已于 2026-08-09 整例
PASS——Bug B ls3 已修复，本次期望整例 PASS，若回归按新缺陷归因）：必跑
`002/003/010/012/014/024/031/038/077`，
推荐低成本加跑 `006/007/011`（§1 mapped、PASS、直接覆盖 Change 1 白名单/
opaque/readdir 表面）。**mapped 但不可调度/不跑**（记入 not-run 列注明原因）：
`001`（≥8GiB；已知底层 block 缺陷，仅用户授权才 combined 复跑）、`004`
（fsgqa 环境门）、`005`（loop+XFS harness 缺口）、`017`（redirect_dir
P2-02 deferred）、`018`/`037`（index=on deferred）、`020`（userns 能力门）、
`021`（F2 out-of-scope + ≥16GiB）、`061`（VM 缺陷关闭）；`019`（fsstress）
仅作可选锁序/死锁观察（用户授权）。Checker 交付 mapped/observed/not-run
三列 + 每例 qemu.log；运行前每例重建镜像（Bug B 未落地）。

**2026-08-10 Checker 返回（OUTCOME B — ACTIONABLE REPAIR BATCH）：** 12 例
11 PASS / 1 FAIL / 0 NOTRUN / 0 HANG；031 整例 PASS（ls3 含，Bug B 未重开）。
**FAIL = overlay/012**（期望 ESTALE，实得 EISDIR），归因 pass_43 Change 1
（lookup_binding 无条件层扫描后重建把 stale-upper 情形落到 lower 目录，
remove_target 的 ESTALE 臂不可达）。完整三字段 + 逐字诊断 + 修复批次见
`components/wave7_cache_consistency_design/pass_43_wave7_cache_consistency_checker.md`
（§4）与 PASS_SLICING pass_43。**NEXT：** 有界 Designer 会签（冻结
lookup_binding 新鲜真值派生中 stale-upper 与真回落 lower 的区分 + remove_target
路由 ESTALE）→ Creator 修复 pass → 单跑 overlay/012（fresh 8 GiB）确认
ESTALE + 无 warn/oops → 复跑 12 例全表确认其余 11 例仍绿。
**2026-08-10 已派发（用户授权）：** `task_designer_wave7_cache_consistency_012_repair_20260810`
（Direct Spawn Lane，agent Pasteur）冻结 stale-upper 区分 + ESTALE 路由 +
验证契约；packet `subagent-tasks/wave7_cache_consistency_design/
task_designer_wave7_cache_consistency_012_repair_20260810_dispatch.md`。
**Designer 会签 ACCEPTED（2026-08-10）：** 冻结 `Binding::is_stale_upper` +
`LookupOutcome` 载体 + `lookup_binding -> Result<LookupOutcome>`（探针在
matches_truth 失败时派生信号）+ `remove_target` step-1 在类型门之前路由
ESTALE（translate_stale_upper_enoent）+ 7 处机械 `.binding`；零 VFS、无新
锁域、无 ktest。**pass_44_wave7_cache_consistency_012_repair Creator 已派发**
（Direct Spawn Lane，agent Euler；packet
`subagent-tasks/wave7_cache_consistency_design/
task_creator_wave7_cache_consistency_012_repair_20260810_dispatch.md`；记录见
PASS_SLICING pass_44）。
**pass_44 Creator ACCEPTED + 编译门 PASSED（2026-08-10）：** diff 对照冻结面
逐项核验（is_stale_upper 逐字、LookupOutcome + 探针 delta、remove_target
step-1 ESTALE 路由、7 处机械 .binding）；容器 cargo check 0 错误（仅预存
uuid_mode 警告）；提交 `cd29d9c17`（8 文件 +112/-26）。**复验 Checker 已派发**
（agent Peirce；step (i) 012 单跑 + step (ii) 12 例全表，期望 12/12 PASS；
packet `subagent-tasks/wave7_cache_consistency_design/
task_checker_wave7_cache_consistency_012_repair_20260810_dispatch.md`）。
**复验返回（2026-08-10，OUTCOME A — VERIFIED ACCEPTANCE）：** step (i)
`overlay/012` 单跑 PASS（ESTALE，serial 无 warn/oops）；step (ii) 12 例全表
**12/12 PASS、0 NOTRUN、0 HANG**（evidence
`run_evidence/pass44_012_repair_20260810/`，13 runs；receipt
`pass_44_wave7_cache_consistency_012_repair_checker.md`）。**pass_43 +
pass_44 gate-accepted**；§8 统一账 012 恢复绿色（20/43）。Wave7 可调度面
剩余未处理项（用户决策）：`041`（show_options 回显，VFS 接口门）、`013`
（ETXTBSY 分歧，已记录）、`001`/`019`（重负载，用户可选）、`028`
（flock 候选未测）；`legacy_fs.rs` 移交决策未做。
**2026-08-10 021 定向重试（user-directed；运行中）：** `overlay/021`（并发
copy-up 炸弹）在 pass_43/44 修复面下重跑——单例 16 GiB 镜像、1500s 预算、
必须报告到达阶段（种植/并发/终态）。packet
`subagent-tasks/wave7_cache_consistency_design/
task_checker_wave7_overlay021_retry_20260810_dispatch.md`（agent Banach）。
**021 重试结果（2026-08-10）：FAIL，与 2026-08-09 同阶段（炸弹种植/前置）同症状**
——4 个 glob `*0/*4/*8/*b` 全部 `No such file or directory` + `find: 'p2'/'p3'`
缺失；无 panic/hang/CORRUPT；pass_43/44 未改变结果或阶段，**非 overlayfs
回归，harness/前置归因不变**。未出现更晚阶段（并发 copy-up 正确性）证据。
证据 `run_evidence/overlay021_retry_20260810/`（16 GiB 镜像、无残留）；收据
`pass_43_021_retry_checker.md`。下一步候选（用户决策）：只读排查种植前置
（fsstress/`xfs_io` 在 ext2 上的文件创建/持久性与上游 `*0/*4/*8/*b` glob 的
命名/可见性是否匹配），而非 overlayfs 代码修复。
**2026-08-10 方案 A（user-directed；运行中）：** 最小 guest 冒烟——临时把
`test/initramfs/src/boot_hello.sh` 换成 fsstress 冒烟脚本（guest 内直跑
`/opt/xfstests/ltp/fsstress -d /tmp/x -p 4 -z -f creat=1 -n 16/-n 256 -v`，
打印 errno/tree/counts），`AUTO_TEST=boot ENABLE_CONFORMANCE_TEST=true
CONFORMANCE_TEST_SUITE=xfstests` 构建含 xfstests 的 initramfs，跑完精确还原。
packet `subagent-tasks/wave7_cache_consistency_design/
task_checker_wave7_fsstress_smoke_20260810_dispatch.md`（agent Hume）。
**冒烟结果（2026-08-10，根因已定）：** guest 内直跑
`/opt/xfstests/ltp/fsstress -d /tmp/x -p 4 -z -f creat=1 -n 16/-n 256 -v`——每进程
`io_setup failed`（恰 4 条），0 个文件、0 个 p{0..3}，`FSSTRESS_RC=0` 是父进程
掩码（子进程 exit(1)）。Asterinas 无 aio `io_setup`（syscall 206）→
`kernel/src/syscall/mod.rs` 默认分支 ENOSYS；fsstress 在任何文件操作前退出。
**这直接解释 021 种植 0 文件 → glob 全空 → overlay 镜像 find 缺失。**
归属：**`内核能力缺口（aio io_setup ENOSYS）`**（与 §8 统一账 021 行一致），**非 overlayfs**；
pass_43/44 既非因也非解。证据 `run_evidence/fsstress_smoke_20260810/`；
收据 `pass_43_021_fsstress_smoke_checker.md`；boot_hello.sh 已精确还原（sha256
匹配），git 无 harness 改动。路由选项（用户决策）：(a) 内核补 `io_setup`/
`io_destroy`（超 overlayfs 范围，需另行授权）；(b) 重打包无 AIO 的 fsstress；
(c) 用 touch 循环等替代播种 021 lower。无 overlayfs Creator 修复指向。
**2026-08-10 定案（user-directed）：不再路由任何选项；021 记为「内核缺少 aio
能力导致 FAIL（io_setup ENOSYS，非 overlayfs）」。Wave7 case 记录收尾。**

0. **061 已关闭（非 overlayfs 缺陷，不修）**：机理记录于
`components/wave7-xfstests-sequencing/overlay061_reinvestigation_20260810.md`
（handoff §8.4；证据 run_evidence/overlay061/ovfs061d_20260810/）。根因 =
通用 VM/page-cache 缺陷（MAP_SHARED mmap 写不标 CachePage dirty → unmount
回写丢失，`kernel/src/vm`）；用户决定不修，061 从 overlayfs wave 关闭。

1. **pass_40 impure + whiteout-cleanup fix IMPLEMENTED, ACCEPTED, VALIDATED
   (2026-08-08):** `overlay/038` PASS (impure set/clear/filter verified);
   `overlay/031` in-scope objectives verified (both `ENOTEMPTY` fixed by the
   pure-upper sweep; rm3 lower-backed publish correct). Receipts
   `pass_40_wave7_impure_cleanup_creator.md` +
   `pass_41_wave7_impure_cleanup_checker.md`; per-case evidence
   `run_evidence/{overlay031,overlay038}/impure_cleanup_20260808/`.
2. **Bug B — base-fs↔overlayfs view coherence (REGISTERED 2026-08-08; 031 ls3
   CLOSED 2026-08-09 — see §8.5):** root cause of 031 ls3 `ENOENT` and the
   031→020 cross-run `_scratch_mkfs` residue. The 031 ls3 portion is FIXED:
   pass_42 (Path-anchored `RealObject.real_path` + dentry-routed
   namespace-mutating upper writes) plus the 2-line VFS visibility widening
   (`e3da18bd9`) produced a whole-case `overlay/031` PASS on 2026-08-09.
   Historical analysis of the original root cause: **Candidate A (primary):** retain the
   layer-root `Path`s at mount (anchors) and route every namespace-mutating
   physical upper write (whiteout publish `mknod`/`link`, copy-up workdir→
   upper `rename`, clear-empty `Exchange`, remove/sweep `unlink`/`rmdir`,
   `new_fs_child`) plus name lookups through the dentry/`Path` layer,
   carrying a dentry-anchored `Path` in `RealObject`; content/metadata ops
   (I/O, xattr incl. impure marker, mode/times, `readdir_at`) stay raw inode.
   Equals Linux semantics; costs = `RealObject` carrier change + `DIR`→
   dentry-children lock order + dentry lifetime. Alternatives B
   (cache-invalidation seam) / C (base readdir read-through) compared in the
   design task. Open sub-question: stale on-disk dirent (031→020) — overlay
   raw-unlink artifact vs ext2 unlink defect; diagnose first. Requires a user
   scope decision (VFS boundary) and a separate design task. Until it lands,
   every case runs on freshly rebuilt images.
3. **041 — `xino=on` mount-option echo gap (REGISTERED, interface gate):**
   option is parsed and xino works (038), but `/proc/self/mounts` prints no
   fs-specific options (no Linux `show_options` equivalent), so the case's
   grep gate always NOTRUNs. Fix requires a procfs/VFS export seam + print in
   `MountsFileOps` (VFS boundary decision); verify the guest `/proc/mounts`
   row layout (overlay source is a path, not `/dev/vde`) before promising the
   case passes.
4. **020 — userns/unshare capability gate:** `unshare -m -p -U` unsupported
   in the kernel; NOTRUN, not an overlayfs fix.
5. **`overlay/013` ETXTBSY:** redesign without any VFS interface change
   (current mechanism fully reverted); not scheduled.
6. **Excluded (user decisions):** `overlay/078` (chattr/fileattr gate) and
   stress cases `001`/`021`/`019`; environment-gated singles
   (`035`/`040`/`027` chattr, `004`/`008`/`015`/`025` fsgqa, `023` chacl)
   remain unscheduled.
7. **Boundaries:** no VFS interface-breaking changes without explicit user
   authorization; `.agents` records stay uncommitted; the pass_40 production
   `.rs` changes (9 files) remain uncommitted pending a user commit decision.
8. **Dispatch lane (NEW, mandatory):** all future Architect/Designer/Creator/
   Checker/Reviewer dispatches use the verified V2 lane — write the packet,
   fill the User Dispatch Turn template, have the user post it, then spawn
   with `fork_turns="1"` and `task_name=<task_id>`. Do not deliver task
   content via spawn payload, followup, or goal text; continuation rounds are
   new dispatch turns. Mechanism facts and smoke evidence: §4; normative
   text: `PROTOCOL.md` §1.3.

## 7. Live File Discipline

- **This file is the live handoff for:** the active Wave7 xfstests tenure.
- **Update rule:** Update this file in place for every Wave7 start decision,
  dispatch, result, repair routing, acceptance, rejection, or escalation.
- **Supersedes / Replaces:**
  `20260804-wave6-documentation-lint_main_agent_handoff.md`, closed / handed
  over on 2026-08-05.

## 8. Case Classification Ledger — 统一用例总账（2026-08-10 修订；43 例）

**43 例统一账 = 原始 38 例（32 可调度 + 6 不可调度）+ 5 例候选增补
（063/066/061/022/028，源自 §8.1 未覆盖扫描，实测结果见 §8.2）。**
每行 = 归属分类（by attribution，fixed-or-not 不是判据）+ 当前状态
（PASS 口径见 §8.5）。当前无任何遗留 FAIL/NOTRUN 归属于 overlayfs 重构本身。

| 用例 | 归属分类 | 当前状态 | 备注/证据 |
| :--- | :--- | :--- | :--- |
| 001 | **底层 block 缺陷**（bio.rs:429 NoMemory unwrap） | FAIL（非 overlayfs） | §8 台账 / `run_evidence/overlay001/` |
| 002 | **行为正确** | PASS | §8.5 |
| 003 | **行为正确** | PASS | §8.5 |
| 004 | **环境门 + 内核缺口**（缺 fsgqa） | NOTRUN | §8 台账 |
| 005 | **harness 缺口**（需 loop+XFS lane） | 不可调度 | §2.2 |
| 006 | **行为正确** | PASS | §8.5 |
| 007 | **行为正确** | PASS | §8.5 |
| 008 | **环境门 + 内核缺口**（缺 fsgqa） | NOTRUN | §8 台账 |
| 009 | **行为正确** | PASS | §8.5 |
| 010 | **overlayfs 缺陷**（whiteout 残留）→ 已修复 | PASS | §8.5 |
| 011 | **行为正确** | PASS | §8.5 |
| 012 | **overlayfs 缺陷**（stale-upper ESTALE）→ 已修复 | **PASS（pass_44 修复后复验通过，2026-08-10）** | §8.5 / pass_43 gate + pass_44 复验 |
| 013 | **VFS 职责**（ETXTBSY，文档化分歧） | 分歧（非 overlayfs 缺陷） | §8 台账 |
| 014 | **overlayfs 缺陷**（lowerdir 顺序 + readdir opaque 屏障）→ 已修复 | PASS | §8.5 |
| 015 | **环境门 + 内核缺口**（缺 fsgqa） | NOTRUN | §8 台账 |
| 016 | **行为正确** | PASS | §8.5 |
| 019 | **行为正确**（fsstress） | PASS | §8.5 |
| 020 | **内核能力缺口**（无 userns/unshare） | NOTRUN | §8 台账 |
| 021 | **内核能力缺口（aio）**（Asterinas 无 `io_setup` → ENOSYS → fsstress 0 文件；2026-08-10 冒烟定因） | FAIL（非 overlayfs；pass_43/44 面下重试同症状） | §8 台账 / pass_43_021_retry_checker.md + fsstress_smoke_checker.md |
| 022 | **行为正确**（overlay 作 upperdir 被拒） | PASS | 候选增补 §8.2 / §8.5 |
| 023 | **环境门 + 内核缺口**（缺 chacl/POSIX ACL） | NOTRUN | §8 台账 |
| 024 | **overlayfs 缺陷**（workdir 清理/残留）→ 已修复 | PASS | §8.5 |
| 025 | **环境门 + 内核缺口**（缺 fsgqa） | NOTRUN | §8 台账 |
| 026 | **overlayfs 缺陷**（xattr errno）→ 已修复 | PASS | §8.5 |
| 027 | **内核能力缺口**（无 fileattr/FS_IOC_*） | NOTRUN | §8 台账 |
| 028 | **未测候选**（flock over copy-up；低-中置信） | 未测 | 候选增补 §8.1 |
| 029 | **行为正确**（嵌套 overlay d_real） | PASS | §8.5 |
| 031 | **overlayfs 缺陷**（whiteout sweep+Bug B）→ 已修复 | **PASS（整例）** | §8.5；Bug B ls3 2026-08-09 已修 |
| 032 | **deferred 功能**（index=on） | 不可调度 | §2.2 |
| 033 | **deferred 功能**（index=on） | 不可调度 | §2.2 |
| 034 | **deferred 功能**（index=on） | 不可调度 | §2.2 |
| 035 | **内核能力缺口**（无 fileattr/FS_IOC_*） | NOTRUN | §8 台账 |
| 037 | **deferred 功能**（index=on） | 不可调度 | §2.2 |
| 038 | **overlayfs 缺陷**（impure 标记）→ 已修复 | PASS | §8.5 |
| 039 | **行为正确**（relatime） | PASS | §8.5 |
| 040 | **内核能力缺口**（无 fileattr/FS_IOC_*） | NOTRUN | §8 台账 |
| 041 | **内核/VFS 缺口**（挂载选项回显 show_options） | NOTRUN | §8 台账 |
| 042 | **deferred 功能**（index=on） | 不可调度 | §2.2 |
| 061 | **非 overlayfs 内核缺陷**（通用 VM/page-cache） | 已关闭（不修） | 候选增补 §8.2/§8.4 |
| 063 | **行为正确**（create-over-whiteout 回归） | PASS | 候选增补 §8.2 / §8.5 |
| 066 | **内核能力缺口**（FS_IOC_GETXATTR ioctl） | FAIL（ioctl 缺口） | 候选增补 §8.2 |
| 077 | **行为正确**（readdir 缓存失效） | PASS | §8.5 |
| 078 | **内核能力缺口**（无 fileattr/FS_IOC_*） | NOTRUN | §8 台账 |

**分类口径汇总（2026-08-10 修正；原「deferred 6」为计数错误，实为 5）：**
overlayfs 缺陷 7（全部已修复 → PASS）/ 行为正确 13 / VFS 职责 1 /
底层 block 1 / 非 overlayfs 内核缺陷（VM）1 /
内核能力缺口 8（含 021：aio `io_setup` ENOSYS）/ 环境门+内核缺口 5 /
harness 缺口 1 / deferred 5 / 未测候选 1 = **43**。§8.5 按「至少一次整例通过」计 20；012 在 pass_43 门
（2026-08-10）曾回归 FAIL（Change 1），经 pass_44 修复 + 复验（12/12 PASS）
后恢复绿色，**当前绿色 = 20/43**。
## 8.1 Un-covered overlay cases (full.list 80 − ledger 43 = 37; feasibility
scan 2026-08-09, read-only, from the packaged suite source; **2026-08-10 修订：
5 个「值得跑」候选（063/066/061/022/028）已并入 §8 统一账，其中
063/066/061/022 实测结果见 §8.2，028 仍未测**）

- **值得跑（可能通过）5**：`063`（create-over-whiteout 无崩溃回归，robust，
  高置信）、`066`（sparse copy-up 后 `diff -qr` 内容一致性；xfs_io 在，
  中高）、`061`（016 的 mmap 写后读变体；需 mmap/page-cache 路径 OK，中）、
  `028`（flock over copy-up；工具在，需锁委托，低-中）、`022`
  （overlay 作 upperdir 的 mount 拒绝；无门，需 mount 已实现该校验，低）。
- **可运行但预计 FAIL 1**：`072`（offline 加 upper 硬链接后 nlink 记账，
  P2-07/deferred 语义未实现）。
- **门控 NOTRUN 36**：index=on（018/036/047/048/065/073）、
  index+nfs_export 文件句柄/嵌套（050-055/058/062/068-071/074）、
  redirect_dir（017/043/049/057/059）、metacopy（060/064）、
  挂载选项门（044 index=on,xino=on；067 xino=off）、fsck.overlay 工具缺失
  （045/046/056）、unionmount 套件缺失（100/101）、fileattr（030/075/076）。
- 结论（2026-08-10 修订）：5 个「值得跑」候选的实测结果为 2 PASS
  （063/022）、1 ioctl 缺口 FAIL（066）、1 关闭（061，非 overlayfs VM 缺陷），
  028 未测；其余未涉及 37 例要么 deferred 功能（index/nfs_export/
  redirect/metacopy/xino 选项），要么内核能力（fileattr/ioctl），要么工具/lane
  缺失（fsck.overlay/unionmount/loop）。

## 8.2 Un-covered candidates — test results (2026-08-09, user-directed; one
run each, fresh 8G images, no HANG)

- `overlay/063` **PASS** — create-over-whiteout 无崩溃回归（rm→whiteout、
  移除 upper whiteout、mkdir over stale whiteout），符合预期。
- `overlay/066` **FAIL** — `FS_IOC_GETXATTR: Inappropriate ioctl for device`
  刷屏：xfs_io 打开文件时探测该 ioctl，Asterinas 未实现 → VFS/ioctl 能力
  缺口，非 sparse copy-up 内容问题（内容断言未实际走到）。
- `overlay/061` **FAIL — 2026-08-10 归因修正并关闭：通用 VM/page-cache
  缺陷，非 overlayfs 缺陷（不修）**。原记录"mmap 写后 copy-up 内容仍为旧值 /
  真实 overlayfs 缺陷"经 3 轮全量打点改为：mmap 写实际发生在 upper VMO，但
  CachePage 未标 dirty → unmount 回写丢失 → after-cycle 读旧值。机理见
  `overlay061_reinvestigation_20260810.md`（§8.4）。
- `overlay/022` **PASS** — overlay 作 upperdir 的二次挂载被正确拒绝
  （`Silence is golden`）；原以为需要补的 mount 校验目前已被现有逻辑挡下。

证据均归档 `run_evidence/{overlay063,overlay066,overlay061,overlay022}/
candidates_20260809/{qemu.log,qemu-serial.log}`；临时 runlist 与生成镜像
已删除。台账更新：Wave7 累计 **PASS 20 / FAIL 4（原始结果口径：001/021/061/
066；其中 061 = 非 overlayfs VM/page-cache 缺陷、§8.4 关闭，066 = ioctl
能力缺口，均非 overlayfs 重构缺陷）**；5 候选中实测 4 例：2 PASS
（063/022）、1 非 overlayfs 内核缺陷（061，关闭）、1 内核 ioctl 缺口（066），
028 未测。

## 8.3 overlay/061 双 copy-up 缺陷定位（2026-08-09；三轮临时打点
`[ovfs061]`/`[ovfs061b]`/`[ovfs061c]`，已全部还原；无代码残留）

061（mmap 写后读一致性）FAIL 根因已从"待归因"升级为**已定位的 overlay
投影/InodeCache 一致性缺陷**：

1. **key 跨 copy-up 不稳定**：`RealObjectKey::from_facts` =
   `from_source(visible_source(facts))`（projection/inode_cache.rs:79）——
   lower-backed 时 key=lower 原始 ino（foo=15），upper-backed 后 key=upper
   原始 ino（17），同一逻辑对象 copy-up 前后换 key，无跨对象唯一性保证。
2. **InodeCache 同 key 双载体**：key 17 已被 harness 的 "358.xfs_io" 文件
   载体占用（0x…c22010），foo 的 upper 投影 `get_or_create(key17)` 仍新建
   载体 0x…5e2810 —— 去重/别名失效（撞车）。
3. **陈旧 lower 载体滞留**：BindingCache 曾把 `(root,foo)` 指向 lower-backed
   载体 A（key15, facts.upper=None）；写意图路径命中 A →
   `ensure_upper_authority` Step 2 看到 upper=None → 第二次完整 promote
   （copy-up #2，`replace_facts` 15→17）。
4. **两次 mmap 落两个不同载体/不同 page cache** → mwrite 写进一个、
   mread/重挂载读另一个 → "This is old news"。

触发执行流程与机制文档：
`components/wave7-xfstests-sequencing/overlay061_bug_trigger_flow_20260809.md`
（含逐步骤 flow、三处打点关键行、候选修复方向）。后续行动 = §6 NEXT
ACTION：继续探究并研究解决此 bug。

## 8.4 overlay/061 归因修正（2026-08-10；主代理亲自全量打点，3 轮单例运行）

- **结论**：`overlay/061` after-cycle FAIL 的根因是**通用 VM/page-cache
  缺陷**：MAP_SHARED mmap 写缺页（`vm_mapping.rs::handle_single_page_fault`
  → `prepare_page`）只置 PTE DIRTY，从不调 `CachePage::set_dirty()`；
  `BackedVmo::flush_dirty_pages` 只回写 `is_dirty()` 页 → unmount 时
  `dirty_pages=0` → mmap 写全部丢失。证据：mwrite 在 upper VMO 上产生
  write fault（PF），unmount flush upper VMO `dirty_pages=0`，after-cycle
  读到旧内容；Linux 同序列（容器真实 overlay + C 程序）after-cycle 为
  `aaaaaaaaaaaaaaaa`。
- **overlayfs 投影侧无缺陷（本场景）**：3 轮均为单载体、单 promote、
  `alias_key` 15→17 OK、binding 不换载体；2026-08-09 的双 copy-up/同 key
  双载体归因未复现（其探针未区分两个 overlay mount 的独立 InodeCache）。
- **in-place mread 读旧数据 = 与 Linux 一致**：copy-up 前建立的 ro 映射
  绑定 lower page cache，copy-up 后仍读 lower（Linux `ovl_mmap` 同样行为）；
  测试 in-place 断言依赖 xfs_io 映射表语义（版本相关），非 Asterinas
  overlayfs 缺陷。
- 详细文档（061 错误机理唯一记录）：
  `components/wave7-xfstests-sequencing/overlay061_reinvestigation_20260810.md`；
  证据：`run_evidence/overlay061/ovfs061d_20260810/`（run1 全量 overlay 探针、
  run2 +PF/+FLUSH、run3 +MUNMAP/+PFA）。所有临时探针已还原，工作树干净。
- **处置（用户决定 2026-08-10）：非 overlayfs 缺陷，不修，061 关闭**；
  归因为通用 VM/page-cache 缺陷（`kernel/src/vm`），与 overlayfs wave 台账
  解耦。

## 8.5 已通过用例总表（2026-08-10 修正；当前状态）

Wave7 至今**整例通过（PASS）共 20 例**（每例至少一次整例 `PASS`；证据见
§4 对应批次与 `run_evidence/<case>/`）：

| 用例 | 首次通过批次 | 当前状态 | 证据 |
| :--- | :--- | :--- | :--- |
| 002 | 单纯用例批次 / Run4（2026-08-07） | PASS | `run_evidence/overlay002/` |
| 003 | 单纯用例批次（2026-08-07） | PASS | `run_evidence/overlay003/` |
| 006 | 单纯用例批次（2026-08-07） | PASS | `run_evidence/overlay006/` |
| 007 | 单纯用例批次（2026-08-07） | PASS | `run_evidence/overlay007/` |
| 009 | 单纯用例批次 / Run3（2026-08-07，首个行为 PASS） | PASS | `run_evidence/overlay009/` |
| 010 | pass_38 修复复跑（2026-08-07） | PASS | `run_evidence/overlay010/rerun_20260807/` |
| 011 | 单纯用例批次（2026-08-07） | PASS | `run_evidence/overlay011/` |
| 012 | pass_38 修复复跑（2026-08-07，stale-upper ESTALE） | PASS（pass_38）→ pass_43 门回归 FAIL → **pass_44 修复后 PASS（2026-08-10）** | `run_evidence/overlay012/rerun_20260807/` + pass44_012_repair_20260810/ |
| 014 | pass_38 修复复跑（2026-08-07，lowerdir 顺序修复后） | PASS | `run_evidence/overlay014/rerun_20260807/` |
| 016 | 单纯用例批次（2026-08-07） | PASS | `run_evidence/overlay016/` |
| 019 | 未测四例补齐（2026-08-09，fsstress 16G） | PASS | `run_evidence/overlay019/untested_four_20260809/` |
| 022 | 候选增补 §8.2（2026-08-10，overlay 作 upperdir 拒绝） | PASS | `run_evidence/overlay022/candidates_20260809/` |
| 024 | pass_38 修复复跑（2026-08-07，workdir 清理） | PASS | `run_evidence/overlay024/rerun_20260807/` |
| 026 | pass_38 修复复跑（2026-08-07，xattr errno） | PASS | `run_evidence/overlay026/rerun_20260807/` |
| 029 | 综合用例批次 pass_39（2026-08-08，嵌套 overlay d_real） | PASS | `run_evidence/overlay029/comprehensive_20260808/` |
| 031 | **整例通过 2026-08-09**（pass_42 + VFS 可见性放宽修复 Bug B ls3 后单跑；pass_41 已 VERIFIED in-scope，仅剩 ls3） | **PASS（整例）** | `run_evidence/overlay031/pass42_visibility_fix_20260809/` |
| 038 | pass_41 修复验证（2026-08-08，impure set/clear/filter） | PASS | `run_evidence/overlay038/impure_cleanup_20260808/` |
| 039 | 单纯用例批次（2026-08-07，relatime） | PASS | `run_evidence/overlay039/` |
| 063 | 候选增补 §8.2（2026-08-09，create-over-whiteout） | PASS | `run_evidence/overlay063/candidates_20260809/` |
| 077 | 综合用例批次 pass_39（2026-08-08，readdir 缓存失效） | PASS | `run_evidence/overlay077/comprehensive_20260808/` |

**031 状态修正（2026-08-10）：** 031 **整例 PASS**（2026-08-09 单跑
`Ran: overlay/031 / Passed all 1 tests`）——Bug B 的 ls3 缺口在 pass_42 +
VFS 可见性放宽（`DirDentry` + `as_dir_dentry_or_err` 放宽至
`pub(in crate::fs)`，commit `e3da18bd9`）后已修复。任何仍把 031 描述为
"ls3 被 Bug B 阻塞 / 整例未通过"的表述均为过时，以本表为准。pass_43 回归
批次中 031 期望整例 PASS；若回归，按新缺陷归因，不再默认是 Bug B 依赖。

**说明：** 本表按"至少一次整例 PASS"口径（20 例）；§8 统一账（43 例）的
归属分类与之正交——例如 031/038 归 overlayfs 缺陷类但已修复并通过。
未整例通过的其余用例（与 §8 统一账一致）：`013`（VFS 职责分歧，非
overlayfs 缺陷）、`001`（底层 block 缺陷）、`021`（内核 aio 能力缺口：`io_setup` ENOSYS）、
`066`（ioctl 能力缺口 FAIL）、`061`（非 overlayfs VM 缺陷，关闭）、
`020`/`041`/`035`/`040`/`027`/`078`/`004`/`008`/`015`/`025`/`023`
（内核能力或环境门 NOTRUN）、`005`/`032`/`033`/`034`/`037`/`042`
（不可调度：loop 或 index=on deferred）、`028`（未测候选）。
