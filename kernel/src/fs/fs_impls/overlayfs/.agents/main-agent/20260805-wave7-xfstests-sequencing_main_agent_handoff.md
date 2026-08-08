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

## 6. Next Actions for the Next Thread (CRITICAL)

1. **pass_40 impure + whiteout-cleanup fix IMPLEMENTED, ACCEPTED, VALIDATED
   (2026-08-08):** `overlay/038` PASS (impure set/clear/filter verified);
   `overlay/031` in-scope objectives verified (both `ENOTEMPTY` fixed by the
   pure-upper sweep; rm3 lower-backed publish correct). Receipts
   `pass_40_wave7_impure_cleanup_creator.md` +
   `pass_41_wave7_impure_cleanup_checker.md`; per-case evidence
   `run_evidence/{overlay031,overlay038}/impure_cleanup_20260808/`.
2. **Bug B — base-fs↔overlayfs view coherence (REGISTERED, out of scope for
   this wave):** root cause of 031 ls3 `ENOENT` and the 031→020 cross-run
   `_scratch_mkfs` residue. Candidate fix directions (overlay upper writes
   routed through the base mount `Path` layer / cache-invalidation seam /
   read-through base readdir) all touch the VFS boundary; requires a user
   scope decision and a separate design task. Until it lands, every case must
   run on freshly rebuilt images.
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

## 7. Live File Discipline

- **This file is the live handoff for:** the active Wave7 xfstests tenure.
- **Update rule:** Update this file in place for every Wave7 start decision,
  dispatch, result, repair routing, acceptance, rejection, or escalation.
- **Supersedes / Replaces:**
  `20260804-wave6-documentation-lint_main_agent_handoff.md`, closed / handed
  over on 2026-08-05.
